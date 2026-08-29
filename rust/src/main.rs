//! Foxglove CLI compatibility binary.

use std::io::{self, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();
    let outcome = foxglove_rust::run_with_prompt_writer(&args, &mut stdin, &mut stdout);

    if let Err(error) = stdout.write_all(&outcome.stdout) {
        if error.kind() != io::ErrorKind::BrokenPipe {
            let _ = writeln!(io::stderr().lock(), "failed to write stdout: {error}");
            return ExitCode::from(1);
        }
    }
    let _ = io::stderr().lock().write_all(&outcome.stderr);
    ExitCode::from(outcome.exit_code)
}
