//! Structured command parsing and offline command execution.

use std::ffi::OsString;
use std::fmt::Write as _;
use std::io::BufRead;

use clap::{Arg, ArgAction, ArgMatches, Command};
use serde_yaml_ng::Value;

use crate::command_spec::{self, Arity, CommandSpec, COMMANDS};
use crate::config::Config;
use crate::help;
use crate::output::Format;

const VERSION: &str = "v1.0.33";
const ROOT_COMMAND: &str = "foxglove";

/// Captured process output and status for one invocation.
#[derive(Debug, Default, Eq, PartialEq)]
pub struct Outcome {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit_code: u8,
}

impl Outcome {
    fn success(stdout: impl Into<Vec<u8>>) -> Self {
        Self {
            stdout: stdout.into(),
            ..Self::default()
        }
    }

    fn failure(stderr: impl Into<Vec<u8>>) -> Self {
        Self {
            stderr: stderr.into(),
            exit_code: 1,
            ..Self::default()
        }
    }
}

pub fn run(
    cli_args: &[OsString],
    stdin: &mut dyn BufRead,
    prompt_writer: &mut dyn std::io::Write,
) -> Outcome {
    if let Some(command) = unknown_root_command(cli_args) {
        return Outcome::failure(format!(
            "Error: unknown command \"{command}\" for \"foxglove\"\nRun 'foxglove --help' for usage.\nError: unknown command \"{command}\" for \"foxglove\"\n"
        ));
    }

    let mut argv = Vec::with_capacity(cli_args.len() + 1);
    argv.push(OsString::from(ROOT_COMMAND));
    argv.extend_from_slice(cli_args);
    let matches = match command().try_get_matches_from(argv) {
        Ok(matches) => matches,
        Err(error) => return Outcome::failure(error.to_string()),
    };
    let mut path = Vec::new();
    let leaf = deepest_matches(&matches, &mut path);

    if matches.get_flag("help") {
        return help_outcome(command_spec::by_path(&path).map_or("root", |spec| spec.id));
    }
    if path.first().is_some_and(|part| part == "help") {
        let target = positionals(leaf);
        if target.is_empty() {
            return help_outcome("root");
        }
        return command_spec::by_path(&target)
            .map_or_else(|| unknown_help_topic(&target), |spec| help_outcome(spec.id));
    }
    if path
        .first()
        .is_some_and(|part| part.starts_with("__complete"))
    {
        return complete(&path[0], &positionals(leaf));
    }

    let Some(spec) = command_spec::by_path(&path) else {
        return Outcome::failure("internal command metadata error\n");
    };
    if let Some(error) = validate_arity(spec, positionals(leaf).len()) {
        return Outcome::failure(error);
    }
    if path.is_empty() || has_children(&path) {
        return help_outcome(spec.id);
    }

    match path
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["version"] => Outcome::success(format!("{VERSION}\n")),
        ["config", action] => run_config(action, &positionals(leaf)),
        ["auth", "configure-api-key"] => configure_api_key(leaf, stdin, prompt_writer),
        ["devices", "list"] => validate_list_format(leaf),
        ["events", "list"] => validate_events(leaf),
        ["attachments", "list"] => validate_attachment_list(leaf),
        ["completion", shell] => completion_script(shell),
        _ => Outcome::failure(format!(
            "This command is not implemented in the Rust migration yet: {}\n",
            path.join(" ")
        )),
    }
}

fn command() -> Command {
    let root = command_spec::by_id("root").expect("root command spec");
    let mut command = build_command(root).bin_name(ROOT_COMMAND);
    command = command
        .arg(global_value("client-id"))
        .arg(global_value("config"))
        .arg(
            Arg::new("debug")
                .long("debug")
                .global(true)
                .action(ArgAction::SetTrue)
                .overrides_with("debug"),
        )
        .arg(
            Arg::new("help")
                .short('h')
                .long("help")
                .global(true)
                .action(ArgAction::SetTrue)
                .overrides_with("help"),
        );
    for name in ["__complete", "__completeNoDesc"] {
        command = command.subcommand(
            Command::new(name).hide(true).disable_help_flag(true).arg(
                Arg::new("positionals")
                    .num_args(0..)
                    .allow_hyphen_values(true)
                    .action(ArgAction::Append),
            ),
        );
    }
    command
}

fn build_command(spec: &'static CommandSpec) -> Command {
    let name = spec.path.last().copied().unwrap_or(ROOT_COMMAND);
    let mut command = Command::new(name)
        .disable_help_flag(true)
        .disable_help_subcommand(true)
        .disable_version_flag(true)
        .subcommand_precedence_over_arg(true);
    for flag in spec.flags {
        // Root flags are installed separately as global arguments.
        if spec.path.is_empty() && matches!(flag.long, "client-id" | "config" | "debug") {
            continue;
        }
        let mut argument = Arg::new(flag.long).long(flag.long);
        if let Some(short) = flag.short {
            argument = argument.short(short);
        }
        argument = if flag.takes_value && flag.multiple {
            argument
                .num_args(1)
                .allow_hyphen_values(true)
                .action(ArgAction::Append)
        } else if flag.takes_value {
            argument
                .num_args(1)
                .allow_hyphen_values(true)
                .action(ArgAction::Set)
                .overrides_with(flag.long)
        } else {
            argument
                .action(ArgAction::SetTrue)
                .overrides_with(flag.long)
        };
        command = command.arg(argument);
    }
    let children = child_specs(spec.path);
    if children.is_empty() {
        command = command.arg(
            Arg::new("positionals")
                .num_args(0..)
                .action(ArgAction::Append),
        );
    } else {
        for child in children {
            command = command.subcommand(build_command(child));
        }
    }
    command
}

fn global_value(name: &'static str) -> Arg {
    Arg::new(name)
        .long(name)
        .global(true)
        .num_args(1)
        .allow_hyphen_values(true)
        .action(ArgAction::Set)
        .overrides_with(name)
}

fn child_specs(parent: &[&str]) -> Vec<&'static CommandSpec> {
    COMMANDS
        .iter()
        .filter(|spec| spec.path.len() == parent.len() + 1 && spec.path.starts_with(parent))
        .collect()
}

fn has_children(path: &[String]) -> bool {
    COMMANDS.iter().any(|spec| {
        spec.path.len() == path.len() + 1
            && spec
                .path
                .iter()
                .zip(path)
                .all(|(left, right)| left == right)
    })
}

fn deepest_matches<'a>(matches: &'a ArgMatches, path: &mut Vec<String>) -> &'a ArgMatches {
    if let Some((name, child)) = matches.subcommand() {
        path.push(name.to_owned());
        deepest_matches(child, path)
    } else {
        matches
    }
}

fn positionals(matches: &ArgMatches) -> Vec<String> {
    matches
        .try_get_many::<String>("positionals")
        .ok()
        .flatten()
        .map_or_else(Vec::new, |values| values.cloned().collect())
}

fn validate_arity(spec: &CommandSpec, count: usize) -> Option<String> {
    let message = match spec.arity {
        Arity::Exact(expected) if expected != count => {
            format!("accepts {expected} arg(s), received {count}")
        }
        Arity::Maximum(maximum) if count > maximum => {
            format!("accepts at most {maximum} arg(s), received {count}")
        }
        Arity::Any | Arity::Exact(_) | Arity::Maximum(_) => return None,
    };
    Some(cobra_error(spec.id, &message))
}

fn cobra_error(id: &str, message: &str) -> String {
    let Some(page) = help::page(id) else {
        return format!("Error: {message}\nError: {message}\n");
    };
    let usage = page
        .stdout
        .split_once("Usage:\n")
        .map_or(page.stdout, |(_, usage)| usage);
    format!("Error: {message}\nUsage:\n{usage}\nError: {message}\n")
}

fn help_outcome(id: &str) -> Outcome {
    match help::page(id) {
        Some(page) => Outcome {
            stdout: page.stdout.as_bytes().to_vec(),
            stderr: page.stderr.as_bytes().to_vec(),
            exit_code: 0,
        },
        None => Outcome::failure(format!("missing production help page for {id}\n")),
    }
}

fn unknown_help_topic(target: &[String]) -> Outcome {
    let topic = target.join(" ");
    Outcome::failure(format!(
        "Error: unknown help topic \"{topic}\"\nRun 'foxglove --help' for usage.\nError: unknown help topic \"{topic}\"\n"
    ))
}

fn unknown_root_command(args: &[OsString]) -> Option<String> {
    let roots = COMMANDS
        .iter()
        .filter(|spec| spec.path.len() == 1)
        .map(|spec| spec.path[0]);
    let mut index = 0;
    while index < args.len() {
        let value = args[index].to_string_lossy();
        if matches!(value.as_ref(), "--debug" | "-h" | "--help") {
            index += 1;
            continue;
        }
        if matches!(value.as_ref(), "--client-id" | "--config") {
            index += 2;
            continue;
        }
        if value.starts_with("--client-id=") || value.starts_with("--config=") {
            index += 1;
            continue;
        }
        if value.starts_with('-') {
            return None;
        }
        if roots.clone().any(|root| root == value)
            || matches!(value.as_ref(), "__complete" | "__completeNoDesc")
        {
            return None;
        }
        return Some(value.into_owned());
    }
    None
}

fn run_config(action: &str, args: &[String]) -> Outcome {
    let key = args.first().map(String::as_str);
    let key = match require_config_key(key) {
        Ok(key) => key,
        Err(error) => return Outcome::failure(error),
    };
    let mut config = match Config::load_default() {
        Ok(config) => config,
        Err(error) => return Outcome::failure(error),
    };
    match action {
        "get" => match config.get_string(config_name(key)) {
            Some(value) => Outcome::success(format!("{value}\n")),
            None => Outcome::failure(format!("No value set for key '{key}'\n")),
        },
        "set" => {
            let Some(value) = args.get(1) else {
                return Outcome::failure(format!(
                    "No value provided. Please provide a value for: {key}\n"
                ));
            };
            config.set(config_name(key), Value::String(value.clone()));
            match config.save() {
                Ok(()) => Outcome {
                    stderr: format!("Configuration updated: {key} = {value}\n").into_bytes(),
                    ..Outcome::default()
                },
                Err(error) => Outcome::failure(error),
            }
        }
        "unset" => {
            if !config.remove(config_name(key)) {
                return Outcome::failure(format!("No value set for key '{key}'\n"));
            }
            match config.save() {
                Ok(()) => Outcome {
                    stderr: format!("Configuration removed: {key}\n").into_bytes(),
                    ..Outcome::default()
                },
                Err(error) => Outcome::failure(error),
            }
        }
        _ => Outcome::failure(format!("unknown config command {action}\n")),
    }
}

fn require_config_key(key: Option<&str>) -> Result<&str, String> {
    let key = key.ok_or_else(|| "No key provided. Valid keys are: project-id\n".to_owned())?;
    if key == "project-id" {
        Ok(key)
    } else {
        Err(format!(
            "Invalid configuration key '{key}'. Valid keys are: project-id\n"
        ))
    }
}

fn config_name(key: &str) -> &str {
    if key == "project-id" {
        "default_project_id"
    } else {
        key
    }
}

fn configure_api_key(
    matches: &ArgMatches,
    stdin: &mut dyn BufRead,
    prompt_writer: &mut dyn std::io::Write,
) -> Outcome {
    let mut config = match Config::load_default() {
        Ok(config) => config,
        Err(error) => return Outcome::failure(error),
    };
    let token = match last_value(matches, "api-key") {
        Some(token) if !token.is_empty() => token,
        _ => {
            if let Err(error) = writeln!(
                prompt_writer,
                "Enter an API key (will be written to {}):",
                config.path().display()
            )
            .and_then(|()| prompt_writer.flush())
            {
                return Outcome::failure(format!("failed to write prompt: {error}\n"));
            }
            let mut input = String::new();
            let bytes_read = match stdin.read_line(&mut input) {
                Ok(bytes_read) => bytes_read,
                Err(error) => return Outcome::failure(format!("failed to read input: {error}\n")),
            };
            if bytes_read == 0 {
                return Outcome::failure("failed to read input: EOF\n");
            }
            let Some(token) = input.split_whitespace().next() else {
                return Outcome::failure("failed to read input: unexpected newline\n");
            };
            token.to_owned()
        }
    };
    let base_url = last_value(matches, "base-url")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "https://api.foxglove.dev".to_owned());
    config.set("auth_type", Value::Number(2.into()));
    config.set("base_url", Value::String(base_url));
    config.set("bearer_token", Value::String(token));
    match config.save() {
        Ok(()) => Outcome::default(),
        Err(error) => Outcome {
            stderr: format!("Configuration failed: {}\n", error.trim_end()).into_bytes(),
            exit_code: 1,
            ..Outcome::default()
        },
    }
}

fn validate_list_format(matches: &ArgMatches) -> Outcome {
    match Format::resolve(
        last_value(matches, "format").as_deref(),
        matches.get_flag("json"),
    ) {
        Ok(_) => Outcome::failure("Phase 3 implements devices list.\n"),
        Err(error) => Outcome::failure(error),
    }
}

fn validate_events(matches: &ArgMatches) -> Outcome {
    if let Some(values) = matches.get_many::<String>("query-field") {
        for value in values {
            if value != "metadata" && value != "properties" {
                return Outcome::failure(format!(
                    "Invalid --query-field value \"{value}\": must be \"metadata\" or \"properties\"\n"
                ));
            }
        }
    }
    Outcome::failure("Phase 3 implements events list.\n")
}

fn validate_attachment_list(matches: &ArgMatches) -> Outcome {
    if last_value(matches, "session-key").is_some() && last_value(matches, "project-id").is_none() {
        return Outcome::failure("--project-id is required when using --session-key\n");
    }
    Outcome::failure("Phase 3 implements attachments list.\n")
}

fn last_value(matches: &ArgMatches, id: &str) -> Option<String> {
    matches
        .get_many::<String>(id)
        .and_then(|mut values| values.next_back().cloned())
        .or_else(|| matches.get_one::<String>(id).cloned())
}

fn completion_script(shell: &str) -> Outcome {
    let script = match shell {
        "bash" => include_bytes!("../completions/bash").as_slice(),
        "fish" => include_bytes!("../completions/fish").as_slice(),
        "powershell" => include_bytes!("../completions/powershell").as_slice(),
        "zsh" => include_bytes!("../completions/zsh").as_slice(),
        _ => return Outcome::failure(format!("unsupported completion shell: {shell}\n")),
    };
    Outcome::success(script)
}

fn complete(mode: &str, args: &[String]) -> Outcome {
    let (context, prefix) = args
        .split_last()
        .map_or((&[][..], ""), |(prefix, context)| {
            (context, prefix.as_str())
        });
    let mut current_path = Vec::new();
    let mut index = 0;
    while index < context.len() {
        let part = &context[index];
        if part.starts_with('-') {
            if flag_takes_value(&current_path, part) {
                index += 2;
            } else {
                index += 1;
            }
            continue;
        }
        let mut candidate = current_path.clone();
        candidate.push(part.clone());
        if command_spec::by_path(&candidate).is_some() {
            current_path = candidate;
        }
        index += 1;
    }
    if context
        .last()
        .is_some_and(|argument| flag_takes_value(&current_path, argument))
    {
        return completion_outcome(&[], 0);
    }
    if prefix.starts_with('-') {
        let candidates: Vec<_> = available_flags(&current_path)
            .into_iter()
            .filter(|flag| flag.starts_with(prefix))
            .collect();
        return completion_outcome(&candidates, 4);
    }
    let parent = current_path.iter().map(String::as_str).collect::<Vec<_>>();
    let children = child_specs(&parent);
    if children.is_empty() {
        return completion_outcome(&[], 0);
    }
    let mut candidates = children
        .into_iter()
        .filter(|spec| {
            spec.path
                .last()
                .is_some_and(|name| name.starts_with(prefix))
        })
        .map(|spec| {
            let name = spec.path.last().expect("child name");
            if mode == "__completeNoDesc" {
                (*name).to_owned()
            } else {
                let description = help::page(spec.id)
                    .and_then(|page| page.stdout.lines().next())
                    .unwrap_or_default();
                format!("{name}\t{description}")
            }
        })
        .collect::<Vec<_>>();
    candidates.sort();
    completion_outcome(&candidates, 4)
}

fn flag_takes_value(path: &[String], argument: &str) -> bool {
    let name = argument
        .strip_prefix("--")
        .and_then(|name| name.split('=').next());
    if matches!(name, Some("client-id" | "config")) {
        return !argument.contains('=');
    }
    let Some(spec) = command_spec::by_path(path) else {
        return false;
    };
    spec.flags
        .iter()
        .any(|flag| name == Some(flag.long) && flag.takes_value && !argument.contains('='))
}

fn available_flags(path: &[String]) -> Vec<String> {
    let mut flags = vec![
        "--client-id".to_owned(),
        "--config".to_owned(),
        "--debug".to_owned(),
        "--help".to_owned(),
    ];
    if let Some(spec) = command_spec::by_path(path) {
        flags.extend(spec.flags.iter().map(|flag| format!("--{}", flag.long)));
    }
    flags.sort();
    flags.dedup();
    flags
}

fn completion_outcome(candidates: &[String], directive: u8) -> Outcome {
    let mut stdout = candidates.join("\n");
    if !stdout.is_empty() {
        stdout.push('\n');
    }
    let _ = writeln!(stdout, ":{directive}");
    let directive_name = if directive == 4 {
        "ShellCompDirectiveNoFileComp"
    } else {
        "ShellCompDirectiveDefault"
    };
    Outcome {
        stdout: stdout.into_bytes(),
        stderr: format!("Completion ended with directive: {directive_name}\n").into_bytes(),
        exit_code: 0,
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::io::Cursor;

    use crate::run;

    fn invoke(args: &[&str]) -> super::Outcome {
        let args = args.iter().map(OsString::from).collect::<Vec<_>>();
        run(&args, &mut Cursor::new(Vec::<u8>::new()))
    }

    #[test]
    fn global_flags_are_accepted_before_and_after_commands() {
        assert_eq!(invoke(&["--debug", "version"]).stdout, b"v1.0.33\n");
        assert_eq!(invoke(&["version", "--debug"]).stdout, b"v1.0.33\n");
    }

    #[test]
    fn help_command_routes_to_the_requested_command() {
        let outcome = invoke(&["help", "devices"]);
        assert_eq!(outcome.exit_code, 0);
        assert!(String::from_utf8(outcome.stdout)
            .unwrap()
            .starts_with("List and manage devices\n"));
    }

    #[test]
    fn hidden_completion_returns_static_commands() {
        let outcome = invoke(&["__completeNoDesc", "dev"]);
        assert_eq!(outcome.stdout, b"devices\n:4\n");
    }
}
