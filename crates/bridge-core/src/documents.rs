use crate::{collaboration, database::query, format::text, history, messages, repo_files};
use anyhow::Result;
use rusqlite::{params, Connection};
use std::path::Path;

pub fn agents(conn: &Connection, project: &str) -> Result<Vec<String>> {
    Ok(query(conn,"SELECT agent FROM role_assignments WHERE project=?1 UNION SELECT agent FROM sessions WHERE project=?1 UNION SELECT agent FROM status WHERE project=?1 UNION SELECT agent FROM claims WHERE project=?1 UNION SELECT agent FROM agent_settings WHERE project=?1 UNION SELECT recipient AS agent FROM messages WHERE project=?1 UNION SELECT sender AS agent FROM messages WHERE project=?1 ORDER BY agent",[project])?.iter().map(|r|text(r,"agent").to_owned()).filter(|a|!a.is_empty() && !["human","all","bridge"].contains(&a.as_str())).collect())
}
fn section(out: &mut String, title: &str, body: &str) {
    out.push_str(&format!(
        "\n## {title}\n\n{}\n",
        if body.is_empty() { "（无）" } else { body }
    ));
}
pub fn render(conn: &Connection, project: &str, time: &str, handover: bool) -> Result<String> {
    let mut out = format!(
        "# {}\n\n项目：{}\n导出时间：{time}\n",
        if handover {
            "项目交接"
        } else {
            "Bridge 协作记录"
        },
        crate::display_path(project)
    );
    if let Some(info) = collaboration::load(conn, project)? {
        section(&mut out, "项目目标", &info.goal);
        section(
            &mut out,
            "模板与章程",
            &format!(
                "模板：{}\n章程版本：v{}\n\n{}",
                info.name, info.version, info.charter
            ),
        );
    } else {
        section(&mut out, "模板与章程", "尚未初始化");
    }
    let roles = collaboration::roles(conn, project)?;
    section(
        &mut out,
        "各方职务",
        &roles
            .iter()
            .map(|(a, s)| format!("- {a}：{s}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    let statuses=query(conn,"SELECT s.*,COALESCE(w.branch,'-') AS branch,COALESCE(w.worktree,s.project) AS worktree FROM status s LEFT JOIN sessions w ON w.id=s.session_id AND w.project=s.project WHERE s.project=? ORDER BY s.agent,s.session_no",[project])?;
    let status = statuses
        .iter()
        .map(|r| {
            format!(
                "### {} #{} · {} · {}\n\n任务：{}\n进度：{}\n卡点：{}\n下一步：{}\n更新时间：{}\n",
                text(r, "agent"),
                r["session_no"],
                text(r, "branch"),
                crate::display_path(text(r, "worktree")),
                text(r, "task"),
                text(r, "progress"),
                text(r, "blockers"),
                text(r, "next_step"),
                text(r, "updated_at")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    section(&mut out, "各会话最新状态", &status);
    let claims = query(
        conn,
        "SELECT * FROM claims WHERE project=? AND (? OR expires_at>=?) ORDER BY agent,path",
        params![project, handover, time],
    )?;
    let claim = claims
        .iter()
        .map(|r| {
            format!(
                "- {} #{}：{}（{}），到期 {}",
                text(r, "agent"),
                r["session_no"],
                text(r, "path"),
                text(r, "note"),
                text(r, "expires_at")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    if handover {
        section(
            &mut out,
            "未完成的事项",
            "以下内容由数据库机械整理，请结合项目文档补全。",
        );
        section(&mut out, "还没释放的认领", &claim);
        let mut unread = String::new();
        let mut recipients = agents(conn, project)?;
        recipients.push("human".into());
        for agent in recipients {
            let rows = messages::unread(conn, project, &agent)?;
            unread.push_str(&format!(
                "### {}\n\n{}\n",
                if agent == "human" {
                    "发给 human 但还没读的消息".into()
                } else {
                    format!("{agent} 的未读消息")
                },
                rows.iter()
                    .map(crate::format::message)
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        section(&mut out, "各方未读消息", &unread);
        section(
            &mut out,
            "项目文档索引",
            &repo_files::index(Path::new(project))?,
        );
    } else {
        section(&mut out, "当前认领", &claim);
        let rows=query(conn,"SELECT * FROM (SELECT * FROM messages WHERE project=? ORDER BY id DESC LIMIT 200) ORDER BY id",[project])?;
        section(
            &mut out,
            "最近 200 条消息",
            &rows
                .iter()
                .map(crate::format::message)
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
    let limit = if handover { 50 } else { 500 };
    let rows=query(conn,"SELECT * FROM (SELECT * FROM events WHERE project IN (?,'global') ORDER BY created_at DESC,id DESC LIMIT ?) ORDER BY created_at,id",params![project,limit])?;
    let history = rows
        .iter()
        .map(history::format_event)
        .collect::<Result<Vec<_>>>()?
        .join("\n");
    section(&mut out, &format!("最近 {limit} 条历史事件"), &history);
    if handover {
        section(&mut out, "风险", "（由接手方补全）");
        section(&mut out, "建议的下一步", "（由接手方补全）");
    }
    Ok(out)
}
