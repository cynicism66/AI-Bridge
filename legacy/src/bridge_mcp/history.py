"""项目交互历史查询及中文格式化。"""

import json

from .store import db


def history_text(project, limit=50, agent=None, kind=None):
    conditions, parameters = ["project IN (?, 'global')"], [project]
    for column, value in (("agent", agent), ("kind", kind)):
        if value is not None:
            conditions.append(f"{column} = ?")
            parameters.append(value)
    parameters.append(limit)
    with db() as conn:
        rows = conn.execute("SELECT * FROM (SELECT * FROM events WHERE " + " AND ".join(conditions)
                            + " ORDER BY created_at DESC, id DESC LIMIT ?) ORDER BY created_at, id",
                            parameters).fetchall()
    return "\n".join(format_event(row) for row in rows)


def format_event(row):
    data = json.loads(row["detail"])
    kind, agent = row["kind"], row["agent"]
    if kind == "status":
        fields = (("任务", "task"), ("进度", "progress"), ("卡点", "blockers"), ("下一步", "next_step"))
        text = "更新状态：" + "｜".join(f"{label} {data[key]}" for label, key in fields if data.get(key))
    elif kind == "message":
        to = "所有人" if data["recipient"] == "all" else data["recipient"]
        text = f"→ {to} 留言：{data['content']}"
    elif kind == "claim":
        note = f"（{data['note']}）" if data.get("note") else ""
        text = f"认领：{data['path']}{note}，到期 {data['expires_at']}"
    elif kind == "release":
        text = f"释放认领：{data['path']}"
    elif kind == "expire":
        text = f"的认领已到期：{data['path']}"
    else:
        text = ("开启" if data["enabled"] else "关闭") + ("全局总开关" if data["scope"] == "global" else "项目")
    return f"[{row['created_at']}] {agent} {text}"
