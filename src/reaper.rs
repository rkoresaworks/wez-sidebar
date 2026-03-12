use std::collections::HashSet;
use std::process::{Command, Stdio};

use crate::config::AppConfig;
use crate::terminal::create_backend;

#[derive(Debug, Clone)]
pub struct ReapedProcess {
    pub pid: u32,
    pub pgid: u32,
    pub tty: String,
    pub elapsed: String,
    pub args: String,
}

/// Parse ps etime format [[DD-]HH:]MM:SS into total seconds
fn parse_etime(etime: &str) -> Option<i64> {
    let etime = etime.trim();
    // Formats: MM:SS, HH:MM:SS, DD-HH:MM:SS
    let (days, rest) = if let Some((d, r)) = etime.split_once('-') {
        (d.parse::<i64>().ok()?, r)
    } else {
        (0, etime)
    };

    let parts: Vec<&str> = rest.split(':').collect();
    match parts.len() {
        2 => {
            let mins = parts[0].parse::<i64>().ok()?;
            let secs = parts[1].parse::<i64>().ok()?;
            Some(days * 86400 + mins * 60 + secs)
        }
        3 => {
            let hours = parts[0].parse::<i64>().ok()?;
            let mins = parts[1].parse::<i64>().ok()?;
            let secs = parts[2].parse::<i64>().ok()?;
            Some(days * 86400 + hours * 3600 + mins * 60 + secs)
        }
        _ => None,
    }
}

/// List claude CLI processes from ps
fn list_claude_processes() -> Vec<(u32, u32, String, String, String)> {
    // pid, pgid, tty, etime, args
    let output = Command::new("ps")
        .args(["-eo", "pid,pgid,tty,etime,args"])
        .stderr(Stdio::null())
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let text = String::from_utf8_lossy(&output.stdout);
    let mut results = Vec::new();

    for line in text.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Parse fixed-width ps output
        let parts: Vec<&str> = line.splitn(5, char::is_whitespace).collect();
        if parts.len() < 5 {
            continue;
        }

        // Filter: only claude CLI processes (not Claude.app)
        let args = parts[4].trim();
        if !is_claude_cli(args) {
            continue;
        }

        let pid = match parts[0].trim().parse::<u32>() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let pgid = match parts[1].trim().parse::<u32>() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let tty = parts[2].trim().to_string();
        let etime = parts[3].trim().to_string();

        results.push((pid, pgid, tty, etime, args.to_string()));
    }

    results
}

/// Check if process args represent a claude CLI (not Claude.app desktop)
fn is_claude_cli(args: &str) -> bool {
    // Exclude: Claude.app, /Applications/Claude.app
    if args.contains("Claude.app") || args.contains("/Applications/Claude") {
        return false;
    }

    // Check @anthropic-ai/claude-code pattern (node script)
    if args.to_lowercase().contains("@anthropic-ai/claude-code") {
        return true;
    }

    // The first token (or second if first is node/bun) must be "claude"
    let mut parts = args.split_whitespace();
    if let Some(first) = parts.next() {
        let first_base = first.rsplit('/').next().unwrap_or(first);
        if first_base == "claude" {
            return true;
        }
        // If first token is a runtime (node, bun, etc.), check second token
        if matches!(first_base, "node" | "bun" | "npx" | "tsx") {
            if let Some(second) = parts.next() {
                let second_base = second.rsplit('/').next().unwrap_or(second);
                if second_base == "claude" {
                    return true;
                }
            }
        }
    }

    false
}

/// Normalize TTY names for comparison
/// ps uses "s000" format, WezTerm uses "/dev/ttys000" format
fn normalize_tty(tty: &str) -> String {
    if tty.starts_with("/dev/tty") {
        // /dev/ttys000 -> s000
        tty.strip_prefix("/dev/tty").unwrap_or(tty).to_string()
    } else {
        tty.to_string()
    }
}

/// Detect and optionally kill orphaned Claude Code processes
pub fn reap_orphans(config: &AppConfig, dry_run: bool) -> Vec<ReapedProcess> {
    let backend = create_backend(&config.backend, config.effective_terminal_path());
    let panes = backend.list_panes();

    // Safety: if terminal is not running (no panes), skip entirely
    if panes.is_empty() {
        return Vec::new();
    }

    // Collect all terminal pane TTYs (normalized)
    let pane_ttys: HashSet<String> = panes
        .iter()
        .map(|p| normalize_tty(&p.tty_name))
        .collect();

    let threshold_secs = config.reaper.threshold_hours * 3600;
    let processes = list_claude_processes();

    let mut reaped = Vec::new();
    let mut killed_pgids: HashSet<u32> = HashSet::new();

    for (pid, pgid, tty, etime, args) in processes {
        // Skip processes attached to a WezTerm pane
        let norm_tty = normalize_tty(&tty);
        if norm_tty != "?" && pane_ttys.contains(&norm_tty) {
            continue;
        }

        // Check elapsed time against threshold
        let elapsed_secs = match parse_etime(&etime) {
            Some(s) => s,
            None => continue,
        };

        if elapsed_secs < threshold_secs {
            continue;
        }

        // Kill by PGID (process group) to get child processes too
        if !killed_pgids.contains(&pgid) {
            if !dry_run {
                // SIGTERM only — no SIGKILL
                let _ = Command::new("kill")
                    .args(["-TERM", &format!("-{}", pgid)])
                    .stderr(Stdio::null())
                    .output();
            }
            killed_pgids.insert(pgid);
        }

        reaped.push(ReapedProcess {
            pid,
            pgid,
            tty,
            elapsed: etime,
            args,
        });
    }

    reaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_etime() {
        assert_eq!(parse_etime("01:23"), Some(83));
        assert_eq!(parse_etime("01:02:03"), Some(3723));
        assert_eq!(parse_etime("01-02:03:04"), Some(93784));
        assert_eq!(parse_etime("   05:30   "), Some(330));
        assert_eq!(parse_etime(""), None);
    }

    #[test]
    fn test_is_claude_cli() {
        assert!(is_claude_cli("/usr/local/bin/node /usr/local/bin/claude --help"));
        assert!(is_claude_cli("claude chat"));
        assert!(is_claude_cli("/home/user/.npm/bin/claude"));
        assert!(is_claude_cli("node /path/to/@anthropic-ai/claude-code/index.js"));
        assert!(!is_claude_cli("/Applications/Claude.app/Contents/MacOS/Claude"));
        assert!(!is_claude_cli("grep claude file.txt"));
    }

    #[test]
    fn test_normalize_tty() {
        assert_eq!(normalize_tty("/dev/ttys000"), "s000");
        assert_eq!(normalize_tty("s000"), "s000");
        assert_eq!(normalize_tty("?"), "?");
    }
}
