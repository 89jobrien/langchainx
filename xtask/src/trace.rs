//! trace — emit skill events to `.ctx/GODMODE.trace.jsonl`.
//!
//! All writes are non-fatal: errors are silently swallowed so a tracing failure
//! never breaks a gate run.

use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

/// Opaque handle returned by [`trace_gate_start`] and consumed by
/// [`trace_gate_end`] / [`trace_gate_error`].
pub struct TraceId {
    gate: String,
    started: Instant,
    /// Path resolved at start time so concurrent tests don't race on env vars.
    path: PathBuf,
}

/// Returns path to `.ctx/GODMODE.trace.jsonl` relative to the repo root.
fn trace_path() -> PathBuf {
    let root = std::env::var("GODMODE_TRACE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            // xtask lives at <root>/xtask — go one level up.
            manifest
                .parent()
                .map(|p| p.join(".ctx"))
                .unwrap_or_else(|| PathBuf::from(".ctx"))
        });
    root.join("GODMODE.trace.jsonl")
}

fn now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (y, mo, d, h, mi, s) = epoch_to_parts(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// Decompose a Unix timestamp (seconds) into (year, month, day, hour, min, sec).
fn epoch_to_parts(mut secs: u64) -> (u64, u64, u64, u64, u64, u64) {
    let s = secs % 60;
    secs /= 60;
    let mi = secs % 60;
    secs /= 60;
    let h = secs % 24;
    secs /= 24;

    let mut days = secs;
    let mut y = 1970u64;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        y += 1;
    }
    let month_days: [u64; 12] = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut mo = 1u64;
    for &md in &month_days {
        if days < md {
            break;
        }
        days -= md;
        mo += 1;
    }
    let d = days + 1;
    (y, mo, d, h, mi, s)
}

fn is_leap(y: u64) -> bool {
    (y.is_multiple_of(4) && !y.is_multiple_of(100)) || y.is_multiple_of(400)
}

fn append_event_to(path: &PathBuf, json: &str) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(f, "{json}");
    }
}

/// Emits a `skill.start` event and returns a [`TraceId`] for later completion.
pub fn trace_gate_start(gate: &str) -> TraceId {
    let path = trace_path();
    let ts = now_iso();
    let json = format!(
        r#"{{"event":"skill.start","gate":{gate_json},"ts":"{ts}"}}"#,
        gate_json = serde_json::to_string(gate).unwrap_or_default(),
        ts = ts,
    );
    append_event_to(&path, &json);
    TraceId {
        gate: gate.to_string(),
        started: Instant::now(),
        path,
    }
}

/// Emits a `skill.complete` event with elapsed `duration_ms`.
pub fn trace_gate_end(id: TraceId) {
    let duration_ms = id.started.elapsed().as_millis();
    let ts = now_iso();
    let json = format!(
        r#"{{"event":"skill.complete","gate":{gate_json},"duration_ms":{duration_ms},"ts":"{ts}"}}"#,
        gate_json = serde_json::to_string(&id.gate).unwrap_or_default(),
        duration_ms = duration_ms,
        ts = ts,
    );
    append_event_to(&id.path, &json);
}

/// Emits a `skill.error` event with up to 10 lines of stderr tail.
pub fn trace_gate_error(id: TraceId, stderr: &str) {
    let duration_ms = id.started.elapsed().as_millis();
    let ts = now_iso();
    let stderr_tail: String = stderr
        .lines()
        .rev()
        .take(10)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");
    let json = format!(
        r#"{{"event":"skill.error","gate":{gate_json},"duration_ms":{duration_ms},"stderr_tail":{stderr_json},"ts":"{ts}"}}"#,
        gate_json = serde_json::to_string(&id.gate).unwrap_or_default(),
        duration_ms = duration_ms,
        stderr_json = serde_json::to_string(&stderr_tail).unwrap_or_default(),
        ts = ts,
    );
    append_event_to(&id.path, &json);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;

    // Serialize env-var mutations across all trace tests.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_trace_dir<F: FnOnce(&std::path::Path)>(f: F) {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().expect("tempdir");
        let ctx = dir.path().join(".ctx");
        fs::create_dir_all(&ctx).unwrap();
        unsafe { std::env::set_var("GODMODE_TRACE_DIR", &ctx) };
        f(&ctx.join("GODMODE.trace.jsonl"));
        unsafe { std::env::remove_var("GODMODE_TRACE_DIR") };
    }

    #[test]
    fn test_trace_gate_start_appends_skill_start() {
        with_trace_dir(|trace_file| {
            let _id = trace_gate_start("fmt-check");
            let contents = fs::read_to_string(trace_file).expect("trace file");
            assert!(contents.contains(r#""event":"skill.start""#));
            assert!(contents.contains(r#""gate":"fmt-check""#));
        });
    }

    #[test]
    fn test_trace_gate_end_appends_skill_complete() {
        with_trace_dir(|trace_file| {
            let id = trace_gate_start("clippy");
            trace_gate_end(id);
            let contents = fs::read_to_string(trace_file).expect("trace file");
            assert!(contents.contains(r#""event":"skill.complete""#));
            assert!(contents.contains(r#""duration_ms""#));
        });
    }

    #[test]
    fn test_trace_gate_error_appends_skill_error_with_stderr_tail() {
        with_trace_dir(|trace_file| {
            let id = trace_gate_start("build");
            let stderr = (1..=15)
                .map(|i| format!("line {i}"))
                .collect::<Vec<_>>()
                .join("\n");
            trace_gate_error(id, &stderr);
            let contents = fs::read_to_string(trace_file).expect("trace file");
            assert!(contents.contains(r#""event":"skill.error""#));
            assert!(contents.contains("line 15"), "should include last lines");
            assert!(!contents.contains("line 1\""), "should not include first lines");
        });
    }

    #[test]
    fn test_epoch_to_parts_known_date() {
        // 2026-05-04T00:00:00Z
        // Days from 1970-01-01: 56 * 365 + 14 leap days (1972..=2024 div by 4) = 20454
        // + Jan(31)+Feb(28)+Mar(31)+Apr(30)+3 = 123 => total 20577 days
        // Seconds: 20577 * 86400 = 1_777_852_800
        let (y, mo, d, h, mi, s) = epoch_to_parts(1_777_852_800);
        assert_eq!(y, 2026);
        assert_eq!(mo, 5);
        assert_eq!(d, 4);
        assert_eq!(h, 0);
        assert_eq!(mi, 0);
        assert_eq!(s, 0);
    }
}
