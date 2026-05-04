//! gate — composable gate runner.
//!
//! `Gate` represents a single named check. `GateRunner` sequences gates and
//! reports results, stopping on first failure unless configured otherwise.

use anyhow::Result;
use xshell::Shell;

/// A single quality gate: a name and a function to run.
pub struct Gate {
    pub name: &'static str,
    run: fn(&Shell) -> Result<()>,
}

impl Gate {
    pub fn new(name: &'static str, run: fn(&Shell) -> Result<()>) -> Self {
        Self { name, run }
    }

    pub fn run(&self, sh: &Shell) -> Result<()> {
        eprintln!("[ gate ] {}", self.name);
        (self.run)(sh)
    }
}

/// Runs a sequence of gates in order, stopping on first failure.
pub fn run_gates(sh: &Shell, gates: &[Gate]) -> Result<()> {
    for gate in gates {
        gate.run(sh)?;
    }
    Ok(())
}
