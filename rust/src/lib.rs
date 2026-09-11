//! Foxglove CLI command execution and supporting libraries.

pub mod api;
mod attachments;
mod auth;
mod cli;
pub mod config;
mod data;
mod devices;
mod event_types;
mod events;
mod extensions;
pub mod format;
pub mod output;
mod pending_imports;
mod projects;
mod recordings;
mod records;
mod runtime;
mod sessions;
mod topics;

use std::ffi::OsString;
use std::io::{BufRead, Write};

pub use cli::Outcome;

/// Parse and execute one CLI invocation without terminating the process.
///
/// # Panics
///
/// Panics if the Tokio runtime cannot be created. Async callers must use
/// [`run_async`] instead of invoking this wrapper from another runtime.
pub fn run(args: &[OsString], stdin: &mut dyn BufRead) -> Outcome {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("build Tokio runtime")
        .block_on(run_async(args, stdin))
}

/// Asynchronously parse and execute one CLI invocation without terminating
/// the process.
pub async fn run_async(args: &[OsString], stdin: &mut dyn BufRead) -> Outcome {
    let mut prompts = Vec::new();
    let mut outcome = cli::run_async(args, stdin, &mut prompts).await;
    if !prompts.is_empty() {
        prompts.extend_from_slice(&outcome.stdout);
        outcome.stdout = prompts;
    }
    outcome
}

/// Execute while writing interactive prompts immediately to the supplied writer.
///
/// # Panics
///
/// Panics if the Tokio runtime cannot be created. Async callers must use
/// [`run_with_prompt_writer_async`] instead of invoking this wrapper from
/// another runtime.
pub fn run_with_prompt_writer(
    args: &[OsString],
    stdin: &mut dyn BufRead,
    prompt_writer: &mut dyn Write,
) -> Outcome {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("build Tokio runtime")
        .block_on(run_with_prompt_writer_async(args, stdin, prompt_writer))
}

/// Asynchronously execute while writing interactive prompts immediately.
pub async fn run_with_prompt_writer_async(
    args: &[OsString],
    stdin: &mut dyn BufRead,
    prompt_writer: &mut dyn Write,
) -> Outcome {
    cli::run_async(args, stdin, prompt_writer).await
}
