//! Rust foundation for the Foxglove CLI migration.

pub mod api;
mod cli;
mod command_spec;
pub mod config;
mod help;
pub mod output;

use std::ffi::OsString;
use std::io::{BufRead, Write};

pub use cli::Outcome;

/// Parse and execute one CLI invocation without terminating the process.
pub fn run(args: &[OsString], stdin: &mut dyn BufRead) -> Outcome {
    let mut prompts = Vec::new();
    let mut outcome = cli::run(args, stdin, &mut prompts);
    if !prompts.is_empty() {
        prompts.extend_from_slice(&outcome.stdout);
        outcome.stdout = prompts;
    }
    outcome
}

/// Execute while writing interactive prompts immediately to the supplied writer.
pub fn run_with_prompt_writer(
    args: &[OsString],
    stdin: &mut dyn BufRead,
    prompt_writer: &mut dyn Write,
) -> Outcome {
    cli::run(args, stdin, prompt_writer)
}
