use chrono::{DateTime, Datelike, Local, Utc};
use std::path::PathBuf;
use std::{fs, process::Command, time::Duration};

use crate::config::expand_tilde;
use crate::types::{KeychainCreds, UsageLimits, UsageResponse};

const USAGE_CACHE_FILE: &str = "usage-cache.json";
const COOLDOWN_SECS: u64 = 600; // 10 minutes

fn get_keychain_credentials() -> Option<String> {
    // Try keyring crate first
    if let Ok(entry) = keyring::Entry::new("Claude Code-credentials", "credentials") {
        if let Ok(password) = entry.get_password() {
            return Some(password);
        }
    }

    // Fallback to security command on macOS
    if cfg!(target_os = "macos") {
        let output = Command::new("security")
            .args([
                "find-generic-password",
                "-s",
                "Claude Code-credentials",
                "-w",
            ])
            .output()
            .ok()?;

        if output.status.success() {
            return Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
    }

    None
}

pub fn load_usage_data() -> UsageLimits {
    let mut result = UsageLimits {
        five_hour: -1,
        weekly: -1,
        sonnet: -1,
        ..Default::default()
    };

    let creds = match get_keychain_credentials() {
        Some(c) => c,
        None => return result,
    };

    let keychain_data: KeychainCreds = match serde_json::from_str(&creds) {
        Ok(d) => d,
        Err(_) => return result,
    };

    let token = &keychain_data.claude_ai_oauth.access_token;
    if token.is_empty() {
        return result;
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();

    let response = client
        .get("https://api.anthropic.com/api/oauth/usage")
        .header("Authorization", format!("Bearer {}", token))
        .header("anthropic-beta", "oauth-2025-04-20")
        .send();

    if let Ok(resp) = response {
        if let Ok(usage) = resp.json::<UsageResponse>() {
            result.five_hour = usage.five_hour.utilization as i32;
            if let Some(ref r) = usage.five_hour.resets_at {
                result.five_hour_reset = calculate_reset_time(r);
            }
            result.weekly = usage.seven_day.utilization as i32;
            if let Some(ref r) = usage.seven_day.resets_at {
                result.weekly_reset = format_reset_day(r);
            }
            if let Some(sonnet) = usage.seven_day_sonnet {
                result.sonnet = sonnet.utilization as i32;
            }
        }
    }

    result
}

fn get_cache_path(data_dir: &str) -> PathBuf {
    expand_tilde(data_dir).join(USAGE_CACHE_FILE)
}

/// Hook から呼ばれる: キャッシュが古ければ API を叩いてキャッシュ更新
pub fn cache_usage_if_stale(data_dir: &str) {
    let cache_path = get_cache_path(data_dir);

    // mtime で10分クールダウン判定
    if let Ok(meta) = fs::metadata(&cache_path) {
        if let Ok(modified) = meta.modified() {
            if modified.elapsed().unwrap_or_default() < Duration::from_secs(COOLDOWN_SECS) {
                return;
            }
        }
    }

    let usage = load_usage_data();

    if usage.five_hour < 0 {
        // API 失敗時もキャッシュファイルを touch してクールダウンを効かせる
        // (ファイル未存在 → 毎回リクエスト → レート制限悪循環を防止)
        if let Some(parent) = cache_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(content) = fs::read_to_string(&cache_path) {
            let _ = fs::write(&cache_path, content);
        } else {
            let _ = fs::write(&cache_path, "{}");
        }
        return;
    }

    if let Ok(json) = serde_json::to_string(&usage) {
        if let Some(parent) = cache_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&cache_path, json);
    }
}

/// TUI から呼ばれる: キャッシュファイルを読むだけ (API は叩かない)
pub fn load_usage_from_cache(data_dir: &str) -> UsageLimits {
    let cache_path = get_cache_path(data_dir);
    match fs::read_to_string(&cache_path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => UsageLimits::default(),
    }
}

fn calculate_reset_time(resets_at: &str) -> String {
    let reset_time = match DateTime::parse_from_rfc3339(resets_at) {
        Ok(dt) => dt.with_timezone(&Utc),
        Err(_) => return String::new(),
    };

    let now = Utc::now();
    let diff = reset_time.signed_duration_since(now);

    if diff <= chrono::Duration::zero() {
        return "soon".to_string();
    }

    let hours = diff.num_hours();
    let mins = diff.num_minutes() % 60;

    if hours > 0 {
        format!("{}h{}m", hours, mins)
    } else {
        format!("{}m", mins)
    }
}

fn format_reset_day(resets_at: &str) -> String {
    let reset_time = match DateTime::parse_from_rfc3339(resets_at) {
        Ok(dt) => dt.with_timezone(&Local),
        Err(_) => return String::new(),
    };

    let weekdays = ["日", "月", "火", "水", "木", "金", "土"];
    let weekday_num = reset_time.weekday().num_days_from_sunday() as usize;
    let weekday = weekdays[weekday_num];

    format!("{}/{}({})", reset_time.month(), reset_time.day(), weekday)
}
