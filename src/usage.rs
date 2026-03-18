use std::fs;
use std::path::PathBuf;

use crate::config::expand_tilde;
use crate::types::UsageLimits;

const USAGE_CACHE_FILE: &str = "usage-cache.json";

fn get_cache_path(data_dir: &str) -> PathBuf {
    expand_tilde(data_dir).join(USAGE_CACHE_FILE)
}

/// TUI から呼ばれる: キャッシュファイルを読むだけ
/// (データは Claude Code の statusline スクリプトが書き出す)
pub fn load_usage_from_cache(data_dir: &str) -> UsageLimits {
    let cache_path = get_cache_path(data_dir);
    match fs::read_to_string(&cache_path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => UsageLimits::default(),
    }
}
