use crate::{
    documents, file_batch::Batch, handover, now, preview, redact, repo_files,
    rules_files::RulesFiles, Bridge,
};
use anyhow::{bail, Result};
use rusqlite::TransactionBehavior;
use std::path::Path;

impl Bridge {
    pub fn transfer_text(
        &self,
        project: &str,
        out: Option<&str>,
        to: Option<&str>,
        yes: bool,
    ) -> Result<String> {
        let root = Path::new(project);
        let destination = repo_files::output(
            root,
            out.unwrap_or(if to.is_some() {
                "docs/HANDOVER.md"
            } else {
                "docs/bridge/协作记录.md"
            }),
        )?;
        let db = if self.database.path.exists() {
            std::fs::canonicalize(&self.database.path)?
        } else {
            self.database.path.clone()
        };
        let normalize = |path: &Path| {
            let text = path.to_string_lossy().replace('\\', "/");
            if cfg!(windows) {
                text.to_lowercase()
            } else {
                text
            }
        };
        let db_text = normalize(&db);
        let destination_text = normalize(&destination);
        if destination_text == db_text
            || ["-wal", "-shm"]
                .iter()
                .any(|suffix| destination_text == format!("{db_text}{suffix}"))
        {
            bail!("输出不能覆盖 Bridge 数据库");
        }
        let mut conn = self.database.open()?;
        let data_version: i64 = conn.query_row("PRAGMA data_version", [], |r| r.get(0))?;
        let tx = conn.transaction()?;
        let time = now()?;
        let plan = to
            .map(|agent| handover::Plan::prepare(&tx, project, agent))
            .transpose()?;
        let mut raw = documents::render(&tx, project, &time, plan.is_some())?;
        if let Some(plan) = &plan {
            raw.push_str(&format!("\n## 交接后的职责与首个任务\n\n接手方：{}。规划、实现、自查和提交全部由接手方负责；可写范围不限制。即使 Bridge 关闭或卸载，也请依据本文件和项目文档继续独立开发。\n\n{}\n", plan.to, plan.template.first_task.content));
        }
        tx.commit()?;
        let doc = redact::scan(&raw);
        let mut stats = doc.counts;
        let mut batch = Batch::default();
        batch.add(destination.clone(), doc.text.as_bytes().to_vec())?;
        let mut preview_body = format!("# 输出文件：{}\n\n{}", destination.display(), doc.text);
        if let Some(plan) = &plan {
            preview_body.push_str(&format!("\n# 交接变更预览\n\n接手方：{}\n关闭 AI：{}\n转发未读原消息：{} 条（广播按原消息 ID 去重）\n章程版本：v{}\n将释放其他 AI 的全部认领，原消息与已读记录保留。\n",plan.to,plan.others.join("、"),plan.forwarded.len(),plan.version));
            let charter = redact::scan(&plan.charter);
            for (kind, n) in charter.counts {
                *stats.entry(kind).or_default() += n;
            }
            preview_body.push_str(&format!("\n# 独立开发章程\n\n{}\n", charter.text));
            if plan.write_rules {
                for name in ["AGENTS.md", "CLAUDE.md"] {
                    repo_files::output(root, name)?;
                }
                RulesFiles::prepare(root, &charter.text)?.append_to(&mut batch)?;
                preview_body
                    .push_str("\n将更新 AGENTS.md 和 CLAUDE.md 的章程标记块，块外内容保持原样。\n");
            }
        }
        let extra = redact::scan(&preview_body);
        for (k, n) in extra.counts {
            *stats.entry(k).or_default() += n;
        }
        let (_, message) = preview::create(root, &extra.text, &batch)?;
        let summary = redact::Redacted {
            text: String::new(),
            counts: stats,
        }
        .summary();
        if !preview::confirm(&format!("{message}{summary}\n"), yes)? {
            return Ok("已取消；仓库文件和协作数据未改变，预览文件保留供查看。".into());
        }
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current: i64 = tx.query_row("PRAGMA data_version", [], |r| r.get(0))?;
        if current != data_version {
            bail!("预览后协作数据发生变化，请重新执行并确认");
        }
        // 再查路径，避免确认期间目录被替换为链接。
        for path in batch.paths() {
            repo_files::output(root, &path.to_string_lossy())?;
        }
        if let Some(plan) = &plan {
            plan.apply(&tx, project, &time)?;
        }
        batch.apply()?;
        if let Err(error) = tx.commit() {
            batch.rollback()?;
            return Err(error.into());
        }
        Ok(format!(
            "{}完成：{}\n{summary}",
            if plan.is_some() { "交接" } else { "导出" },
            redact::scan(&destination.to_string_lossy()).text
        ))
    }
}
