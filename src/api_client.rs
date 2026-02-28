use std::collections::HashMap;
use std::time::Duration;

use serde::Deserialize;

use crate::types::{Session, SessionsFile};

#[derive(Deserialize)]
struct ApiSessionsResponse {
    sessions: Vec<Session>,
}

/// EC2 サーバーからセッション一覧を取得
/// 失敗時は None を返す（呼び出し元でローカルキャッシュにフォールバック）
pub fn fetch_sessions(api_url: &str) -> Option<SessionsFile> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .ok()?;

    let url = format!("{}/api/sessions", api_url.trim_end_matches('/'));
    let resp = client.get(&url).send().ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let api_resp: ApiSessionsResponse = resp.json().ok()?;

    let mut sessions = HashMap::new();
    for s in api_resp.sessions {
        sessions.insert(s.session_id.clone(), s);
    }

    Some(SessionsFile {
        sessions,
        updated_at: chrono::Utc::now().to_rfc3339(),
    })
}
