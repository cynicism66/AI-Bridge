//! 仅桌面调用；数据库是主存储，文件先刷盘，调用方才保存 app.json 游标。
use crate::{
    database::query,
    record_frames::{self, Record, HEADER},
    Bridge,
};
use anyhow::{bail, Result};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::{Seek, SeekFrom, Write},
    path::Path,
};

impl Bridge {
    pub fn sync_record_copy(&self, project: &str) -> Result<Option<i64>> {
        let state = self.switch_state(Some(project))?;
        if !state.global_enabled || !state.project_enabled {
            return Ok(None);
        }
        let root = Path::new(project);
        let dir = crate::repo_files::output(root, ".bridge")?;
        if !root.is_dir() {
            bail!("项目文件夹不存在");
        }
        let rows = query(
            &self.database.open()?,
            "SELECT * FROM events WHERE project IN (?,'global') ORDER BY created_at,id",
            [project],
        )?;
        let mut months: BTreeMap<String, Vec<Record>> = BTreeMap::new();
        let mut max_id = 0;
        for row in rows {
            let id = row["id"]
                .as_i64()
                .ok_or_else(|| anyhow::anyhow!("事件 id 无效"))?;
            let stamp = row["created_at"].as_str().unwrap_or("");
            let time = chrono::NaiveDateTime::parse_from_str(stamp, crate::TIME_FMT)?;
            let body = format!(
                "{}\n",
                crate::history::format_event(&row)?.replace('\n', "\n  ")
            );
            months
                .entry(time.format("%Y-%m").to_string())
                .or_default()
                .push(Record {
                    id,
                    stamp: stamp.into(),
                    body,
                });
            max_id = max_id.max(id);
        }
        fs::create_dir_all(&dir)?;
        for (month, records) in months {
            let path = crate::repo_files::output(root, &format!(".bridge/协作记录-{month}.md"))?;
            reconcile(&path, records)?;
        }
        Ok(Some(max_id))
    }
}
fn reconcile(path: &Path, records: Vec<Record>) -> Result<()> {
    let old = crate::file_batch::original(path)?;
    let (existing, complete) = match &old {
        Some(bytes) => record_frames::decode(bytes)?,
        None => (vec![], 0),
    };
    let mut by_id: BTreeMap<i64, Record> = existing.into_iter().map(|r| (r.id, r)).collect();
    for record in records {
        if let Some(previous) = by_id.get(&record.id) {
            if previous != &record {
                bail!("副本事件与数据库不一致，已保留原文件，请核对数据库是否被替换");
            }
        } else {
            by_id.insert(record.id, record);
        }
    }
    let mut ordered: Vec<_> = by_id.into_values().collect();
    ordered.sort_by(|a, b| (&a.stamp, a.id).cmp(&(&b.stamp, b.id)));
    let mut bytes = HEADER.as_bytes().to_vec();
    for record in ordered {
        bytes.extend(record.encode()?);
    }
    if old.as_ref() == Some(&bytes) {
        return Ok(());
    }
    if let Some(old) = old {
        if bytes.starts_with(&old[..complete]) {
            let mut file = OpenOptions::new().write(true).open(path)?;
            file.set_len(complete as u64)?;
            file.seek(SeekFrom::End(0))?;
            file.write_all(&bytes[complete..])?;
            file.sync_all()?;
            return Ok(());
        }
    }
    // 初建或较早时间的补记用原子替换；正常增量仅追加，不丢已有记录。
    crate::atomic_file::write(path, &bytes)
}
