use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=FOXGLOVE_VERSION");
    watch_git_metadata();
    let version = std::env::var("FOXGLOVE_VERSION")
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(git_tag)
        .unwrap_or_else(|| format!("v{}", env!("CARGO_PKG_VERSION")));
    println!("cargo:rustc-env=FOXGLOVE_VERSION={version}");
}

fn watch_git_metadata() {
    // `--git-common-dir` finds the main repository's tag refs even when Cargo
    // runs from a linked worktree, where `--git-dir` is worktree-local.
    let Some(common_dir) = git_output("rev-parse", "--git-common-dir") else {
        return;
    };
    for path in ["HEAD", "packed-refs", "refs/tags"] {
        println!("cargo:rerun-if-changed={common_dir}/{path}");
    }
}

fn git_tag() -> Option<String> {
    let output = Command::new("git")
        .args(["describe", "--tags", "--exact-match", "HEAD"])
        .output()
        .ok()?;
    output.status.success().then_some(())?;
    String::from_utf8(output.stdout)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn git_output(command: &str, argument: &str) -> Option<String> {
    let output = Command::new("git")
        .args([command, argument])
        .output()
        .ok()?;
    output.status.success().then_some(())?;
    String::from_utf8(output.stdout)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}
