//! audit — cargo audit gate.

use anyhow::{Context, Result};
use xshell::{Shell, cmd};

/// Advisory IDs to ignore — transitive deps with no available fix.
/// Review periodically and remove when upstream provides a fix.
const IGNORED_ADVISORIES: &[&str] = &[
    "RUSTSEC-2025-0140", // gix-date: non-utf8 string (needs gix >= 0.74)
    "RUSTSEC-2025-0021", // gix-features: SHA-1 collision (needs gix >= 0.74)
    "RUSTSEC-2023-0071", // rsa: Marvin timing attack (no fix available)
];

/// `cargo audit --ignore <ids...>`
pub fn audit(sh: &Shell) -> Result<()> {
    let mut args = vec!["audit".to_string()];
    for id in IGNORED_ADVISORIES {
        args.push("--ignore".to_string());
        args.push(id.to_string());
    }
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    cmd!(sh, "cargo {args_ref...}")
        .run()
        .context("cargo audit failed")
}
