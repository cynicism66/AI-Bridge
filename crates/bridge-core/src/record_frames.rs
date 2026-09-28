//! 带长度的隐藏边界防止消息中的 Markdown/伪标记被误认成记录边界。
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

pub const HEADER: &str = "# AI Bridge 协作记录\n\n这是 AI Bridge 自动生成的本地副本，未打码，不要提交到公开仓库；想要打码后的版本请用“导出”。请勿手动修改自动记录。\n\n";
const START: &str = "<!-- bridge-event ";
const END: &[u8] = b"<!-- /bridge-event -->\n";
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub id: i64,
    pub stamp: String,
    pub body: String,
}
#[derive(Serialize, Deserialize)]
struct Meta {
    id: i64,
    stamp: String,
    bytes: usize,
}
impl Record {
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut bytes = format!(
            "{START}{} -->\n",
            serde_json::to_string(&Meta {
                id: self.id,
                stamp: self.stamp.clone(),
                bytes: self.body.len(),
            })?
        )
        .into_bytes();
        bytes.extend(self.body.as_bytes());
        bytes.extend(END);
        Ok(bytes)
    }
}
pub fn decode(bytes: &[u8]) -> Result<(Vec<Record>, usize)> {
    if !bytes.starts_with(HEADER.as_bytes()) {
        bail!("协作副本文件头不匹配，已保留原文件，请移开后重试");
    }
    let mut pos = HEADER.len();
    let mut records = Vec::new();
    while pos < bytes.len() {
        let tail = &bytes[pos..];
        if tail.len() < START.len() && START.as_bytes().starts_with(tail) {
            break;
        }
        if !tail.starts_with(START.as_bytes()) {
            bail!("协作副本记录边界损坏，已保留原文件");
        }
        let Some(end) = tail.iter().position(|&b| b == b'\n') else {
            break;
        };
        let header = std::str::from_utf8(&tail[START.len()..end])?;
        let meta: Meta =
            serde_json::from_str(header.strip_suffix(" -->").context("副本记录头损坏")?)?;
        let body_start = pos + end + 1;
        let body_end = body_start.checked_add(meta.bytes).context("副本长度无效")?;
        let next = body_end.checked_add(END.len()).context("副本长度无效")?;
        if next > bytes.len() {
            break;
        }
        if &bytes[body_end..next] != END {
            bail!("协作副本记录尾损坏，已保留原文件");
        }
        records.push(Record {
            id: meta.id,
            stamp: meta.stamp,
            body: std::str::from_utf8(&bytes[body_start..body_end])?.into(),
        });
        pos = next;
    }
    Ok((records, pos))
}
