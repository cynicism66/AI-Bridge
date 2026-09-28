use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectSummary {
    pub project: String,
    pub enabled: bool,
    pub initialized: bool,
    pub unread: i64,
    pub last: Option<String>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Charter {
    pub version: i64,
    pub template: String,
    pub goal: String,
    pub summary: String,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Agent {
    pub agent: String,
    pub role: Option<String>,
    pub enabled: bool,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Session {
    pub older: i64,
    pub agent: String,
    pub session_no: i64,
    pub branch: String,
    pub worktree: String,
    pub task: String,
    pub progress: String,
    pub blockers: String,
    pub next_step: String,
    pub updated_at: String,
    pub last_active: String,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Claim {
    pub path: String,
    pub agent: String,
    pub session_no: i64,
    pub note: String,
    pub expires_at: String,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectDetail {
    pub charter: Option<Charter>,
    pub agents: Vec<Agent>,
    pub sessions: Vec<Session>,
    pub claims: Vec<Claim>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub id: i64,
    pub project: String,
    pub sender: String,
    pub recipient: String,
    pub content: String,
    pub created_at: String,
    pub via: String,
    #[serde(default)]
    pub receipts: Vec<ReadReceipt>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReadReceipt {
    pub agent: String,
    pub read: bool,
    pub read_at: Option<String>,
}
#[derive(Serialize, Debug)]
pub struct MessagePage {
    pub messages: Vec<Message>,
    pub before: Option<i64>,
}
pub enum HumanVia {
    Cli,
    Gui,
}
