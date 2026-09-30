//! Offline command discovery. `cli search` ranks commands against a query and
//! `cli describe` reports one command's arguments, both as JSON derived from
//! the command tree.

use std::any::TypeId;
use std::collections::{BTreeSet, HashMap};

use clap::{Arg, ArgAction, Command};
use serde::Serialize;
use serde_json::Value;

use crate::output::render_json;
use crate::Outcome;

const MAX_RESULTS: usize = 5;
const STOP_WORDS: &[&str] = &[
    "a", "all", "an", "and", "by", "can", "do", "for", "from", "how", "i", "in", "into", "is",
    "it", "me", "my", "of", "on", "or", "some", "that", "the", "this", "to", "want", "with",
];
/// Weights for matches in the command path, summary, long description, and
/// the context of parent summaries and argument help, in that order.
const FIELD_WEIGHTS: [f64; 4] = [8.0, 5.0, 2.0, 0.5];
const PREFIX_WEIGHT: f64 = 0.5;

#[derive(Serialize)]
struct CommandSummary {
    command: String,
    summary: String,
}

#[derive(Serialize)]
struct CommandDescription {
    command: String,
    summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    usage: String,
    arguments: Vec<ArgumentDescription>,
    options: Vec<OptionDescription>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    groups: Vec<GroupDescription>,
    subcommands: Vec<CommandSummary>,
}

#[derive(Serialize)]
struct ArgumentDescription {
    name: String,
    #[serde(rename = "type")]
    kind: ValueType,
    required: bool,
    multiple: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OptionDescription {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    short: Option<String>,
    usage: String,
    #[serde(rename = "type")]
    kind: ValueType,
    required: bool,
    multiple: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    default: Option<Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    possible_values: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    global: bool,
}

#[derive(Serialize)]
struct GroupDescription {
    options: Vec<String>,
    required: bool,
    multiple: bool,
}

#[derive(Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
enum ValueType {
    Boolean,
    Integer,
    Number,
    String,
}

struct Document {
    command: String,
    summary: String,
    fields: [Vec<String>; 4],
}

/// Rank every executable command against `query` and return the best matches.
pub(crate) fn search(mut root: Command, query: &str) -> Outcome {
    root.build();
    let mut documents = Vec::new();
    collect_documents(&root, &mut Vec::new(), &mut Vec::new(), &mut documents);
    let results = rank(&documents, query)
        .into_iter()
        .map(|document| CommandSummary {
            command: document.command.clone(),
            summary: document.summary.clone(),
        })
        .collect::<Vec<_>>();
    json_outcome(&results)
}

/// Describe the command at `path`, which may be given as separate words or as
/// one string, with or without the leading root command name.
pub(crate) fn describe(mut root: Command, path: &[String]) -> Outcome {
    root.build();
    let mut words = path
        .iter()
        .flat_map(|part| part.split_whitespace())
        .collect::<Vec<_>>();
    if words.first() == Some(&root.get_name()) {
        words.remove(0);
    }
    let mut command = &root;
    for (depth, word) in words.iter().enumerate() {
        let Some(next) = subcommands(command).find(|child| child.get_name() == *word) else {
            return unknown_command(command, &words[..depth], word);
        };
        command = next;
    }
    json_outcome(&description(command, &words))
}

fn json_outcome(value: &impl Serialize) -> Outcome {
    let mut stdout = Vec::new();
    match render_json(&mut stdout, value) {
        Ok(()) => Outcome::success(stdout),
        Err(error) => Outcome::failure(format!("failed to render JSON: {error}\n")),
    }
}

fn unknown_command(parent: &Command, parent_path: &[&str], word: &str) -> Outcome {
    let names = subcommands(parent)
        .map(Command::get_name)
        .collect::<Vec<_>>();
    let parent = command_name(parent_path);
    let expected = if names.is_empty() {
        format!("`{parent}` has no subcommands")
    } else {
        format!("`{parent}` has: {}", names.join(", "))
    };
    Outcome::failure(format!(
        "unknown command {word:?}; {expected}\nUse `foxglove cli search <QUERY>` to find a command\n"
    ))
}

/// Subcommands a caller can run, excluding clap's generated `help`.
fn subcommands(command: &Command) -> impl Iterator<Item = &Command> {
    command
        .get_subcommands()
        .filter(|child| child.get_name() != "help" && !child.is_hide_set())
}

fn command_name(path: &[&str]) -> String {
    std::iter::once(crate::cli::ROOT_COMMAND)
        .chain(path.iter().copied())
        .collect::<Vec<_>>()
        .join(" ")
}

fn about(command: &Command) -> String {
    command
        .get_about()
        .map(ToString::to_string)
        .unwrap_or_default()
}

/// Arguments a caller can pass to `command`, including global options that
/// the root command propagates to every subcommand.
fn arguments(command: &Command) -> impl Iterator<Item = &Arg> {
    command
        .get_arguments()
        .filter(|arg| arg.get_id() != "help" && !arg.is_hide_set())
}

fn description(command: &Command, path: &[&str]) -> CommandDescription {
    let mut usage_command = command.clone();
    let usage = usage_command.render_usage().to_string();
    let usage = usage.strip_prefix("Usage: ").unwrap_or(&usage).trim();
    let (positionals, flags): (Vec<&Arg>, Vec<&Arg>) =
        arguments(command).partition(|arg| arg.is_positional());
    CommandDescription {
        command: command_name(path),
        summary: about(command),
        description: command.get_long_about().map(ToString::to_string),
        usage: usage.to_owned(),
        arguments: positionals.into_iter().map(describe_argument).collect(),
        options: flags.into_iter().map(describe_option).collect(),
        groups: command
            .get_groups()
            .filter_map(|group| {
                let mut group = group.clone();
                let multiple = group.is_multiple();
                let required = group.is_required_set();
                // Derived `Args` structs add an unconstrained group of their
                // fields, which describes nothing a caller must know.
                (required || !multiple).then(|| GroupDescription {
                    options: group
                        .get_args()
                        .filter_map(|id| command.get_arguments().find(|arg| arg.get_id() == id))
                        .map(argument_name)
                        .collect(),
                    required,
                    multiple,
                })
            })
            .collect(),
        subcommands: subcommands(command)
            .map(|child| {
                let mut child_path = path.to_vec();
                child_path.push(child.get_name());
                CommandSummary {
                    command: command_name(&child_path),
                    summary: about(child),
                }
            })
            .collect(),
    }
}

fn describe_argument(arg: &Arg) -> ArgumentDescription {
    ArgumentDescription {
        name: argument_name(arg),
        kind: value_type(arg),
        required: arg.is_required_set(),
        multiple: is_multiple(arg),
        description: arg.get_help().map(ToString::to_string),
    }
}

fn describe_option(arg: &Arg) -> OptionDescription {
    let name = argument_name(arg);
    let kind = value_type(arg);
    let possible_values = arg
        .get_possible_values()
        .iter()
        .filter(|value| !value.is_hide_set())
        .map(|value| value.get_name().to_owned())
        .collect::<Vec<_>>();
    let usage = if kind == ValueType::Boolean {
        if arg.get_num_args().is_some_and(|range| range.takes_values()) {
            format!("{name}[=true|false]")
        } else {
            name.clone()
        }
    } else if possible_values.is_empty() {
        format!("{name} <{}>", value_name(arg))
    } else {
        format!("{name} <{}>", possible_values.join("|"))
    };
    let defaults = arg
        .get_default_values()
        .iter()
        .map(|value| typed_value(&value.to_string_lossy(), kind))
        .collect::<Vec<_>>();
    OptionDescription {
        short: arg.get_short().map(|short| format!("-{short}")),
        usage,
        kind,
        required: arg.is_required_set(),
        multiple: is_multiple(arg),
        default: match defaults.len() {
            0 => None,
            1 => defaults.into_iter().next(),
            _ => Some(Value::Array(defaults)),
        },
        possible_values,
        description: arg.get_help().map(ToString::to_string),
        global: arg.is_global_set(),
        name,
    }
}

fn argument_name(arg: &Arg) -> String {
    if arg.is_positional() {
        value_name(arg)
    } else if let Some(long) = arg.get_long() {
        format!("--{long}")
    } else {
        arg.get_short()
            .map_or_else(|| arg.get_id().to_string(), |short| format!("-{short}"))
    }
}

fn value_name(arg: &Arg) -> String {
    arg.get_value_names()
        .and_then(|names| names.first())
        .map_or_else(|| arg.get_id().as_str().to_uppercase(), ToString::to_string)
}

fn value_type(arg: &Arg) -> ValueType {
    let id = arg.get_value_parser().type_id();
    if id == TypeId::of::<bool>() {
        ValueType::Boolean
    } else if [
        TypeId::of::<i64>(),
        TypeId::of::<u64>(),
        TypeId::of::<usize>(),
    ]
    .into_iter()
    .any(|integer| id == integer)
    {
        ValueType::Integer
    } else if id == TypeId::of::<f64>() {
        ValueType::Number
    } else {
        ValueType::String
    }
}

fn typed_value(value: &str, kind: ValueType) -> Value {
    match kind {
        ValueType::String => Value::from(value),
        _ => serde_json::from_str(value).unwrap_or_else(|_| Value::from(value)),
    }
}

fn is_multiple(arg: &Arg) -> bool {
    matches!(arg.get_action(), ArgAction::Append)
        || arg
            .get_num_args()
            .is_some_and(|range| range.max_values() > 1)
}

fn collect_documents<'a>(
    command: &'a Command,
    path: &mut Vec<&'a str>,
    parents: &mut Vec<String>,
    documents: &mut Vec<Document>,
) {
    let children = subcommands(command).collect::<Vec<_>>();
    if children.is_empty() {
        if !path.is_empty() {
            documents.push(document(command, path, parents));
        }
        return;
    }
    if !path.is_empty() {
        parents.push(about(command));
    }
    for child in children {
        path.push(child.get_name());
        collect_documents(child, path, parents, documents);
        path.pop();
    }
    if !path.is_empty() {
        parents.pop();
    }
}

fn document(command: &Command, path: &[&str], parents: &[String]) -> Document {
    let summary = about(command);
    let long_about = command
        .get_long_about()
        .map(ToString::to_string)
        .unwrap_or_default();
    let mut context = parents.join(" ");
    // Global options are on every command, so they would only add noise.
    for arg in arguments(command).filter(|arg| !arg.is_global_set()) {
        context.push(' ');
        context.push_str(&argument_name(arg));
        if let Some(help) = arg.get_help() {
            context.push(' ');
            context.push_str(&help.to_string());
        }
    }
    Document {
        command: command_name(path),
        fields: [
            tokens(&path.join(" ")),
            tokens(&summary),
            tokens(&long_about),
            tokens(&context),
        ],
        summary,
    }
}

/// Split text into lowercase words, dropping a plural `s` so that `dataset`
/// and `datasets` match.
fn tokens(text: &str) -> Vec<String> {
    words(text).map(|word| stem(&word)).collect()
}

fn words(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
}

fn stem(word: &str) -> String {
    match word.strip_suffix('s') {
        Some(stem) if word.len() > 3 && !stem.ends_with('s') => stem.to_owned(),
        _ => word.to_owned(),
    }
}

fn query_terms(query: &str) -> Vec<String> {
    let all = words(query).collect::<Vec<_>>();
    let meaningful = all
        .iter()
        .filter(|word| !STOP_WORDS.contains(&word.as_str()))
        .collect::<Vec<_>>();
    let chosen = if meaningful.is_empty() {
        all.iter().collect()
    } else {
        meaningful
    };
    let mut terms = Vec::new();
    for word in chosen {
        let term = stem(word);
        if !terms.contains(&term) {
            terms.push(term);
        }
    }
    terms
}

/// Score each document by the query terms it contains, weighting matches by
/// field and by how rare the matched word is, and favoring documents that
/// match more of the query.
fn rank<'a>(documents: &'a [Document], query: &str) -> Vec<&'a Document> {
    let terms = query_terms(query);
    if terms.is_empty() {
        return Vec::new();
    }
    let mut frequency = HashMap::<&str, usize>::new();
    for document in documents {
        let unique = document
            .fields
            .iter()
            .flatten()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        for token in unique {
            *frequency.entry(token).or_default() += 1;
        }
    }
    let total = count(documents.len());
    let rarity = |token: &str| {
        let occurrences = count(frequency.get(token).copied().unwrap_or_default());
        (1.0 + (total - occurrences + 0.5) / (occurrences + 0.5)).ln()
    };
    let mut scored = documents
        .iter()
        .filter_map(|document| {
            let mut score = 0.0;
            let mut matched = 0;
            for term in &terms {
                let term_score = document
                    .fields
                    .iter()
                    .zip(FIELD_WEIGHTS)
                    .map(|(tokens, weight)| {
                        weight
                            * tokens
                                .iter()
                                .filter_map(|token| {
                                    match_weight(term, token).map(|quality| quality * rarity(token))
                                })
                                .fold(0.0, f64::max)
                    })
                    .sum::<f64>();
                if term_score > 0.0 {
                    matched += 1;
                    score += term_score;
                }
            }
            (matched > 0).then(|| (score * count(matched) / count(terms.len()), document))
        })
        .collect::<Vec<_>>();
    scored.sort_by(|(left_score, left), (right_score, right)| {
        right_score
            .total_cmp(left_score)
            .then_with(|| left.command.cmp(&right.command))
    });
    scored
        .into_iter()
        .take(MAX_RESULTS)
        .map(|(_, document)| document)
        .collect()
}

fn match_weight(term: &str, token: &str) -> Option<f64> {
    if token == term {
        Some(1.0)
    } else if term.len() >= 3 && token.starts_with(term) {
        Some(PREFIX_WEIGHT)
    } else {
        None
    }
}

fn count(value: usize) -> f64 {
    f64::from(u32::try_from(value).unwrap_or(u32::MAX))
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::io::Cursor;

    use serde_json::Value;

    fn invoke(args: &[&str]) -> crate::Outcome {
        let args = args.iter().map(OsString::from).collect::<Vec<_>>();
        crate::run(&args, &mut Cursor::new(Vec::<u8>::new()))
    }

    fn json(args: &[&str]) -> Value {
        let outcome = invoke(args);
        assert_eq!(
            outcome.exit_code,
            0,
            "{args:?}: {}",
            String::from_utf8_lossy(&outcome.stderr)
        );
        serde_json::from_slice(&outcome.stdout).unwrap()
    }

    fn search(query: &str) -> Vec<String> {
        json(&["cli", "search", query])
            .as_array()
            .unwrap()
            .iter()
            .map(|result| result["command"].as_str().unwrap().to_owned())
            .collect()
    }

    #[test]
    fn search_returns_at_most_five_commands_with_summaries() {
        let results = json(&["cli", "search", "list"]);
        let results = results.as_array().unwrap();
        assert_eq!(results.len(), 5);
        for result in results {
            let object = result.as_object().unwrap();
            assert_eq!(
                object.keys().collect::<Vec<_>>(),
                ["command", "summary"],
                "{result}"
            );
            assert!(!object["summary"].as_str().unwrap().is_empty(), "{result}");
        }
    }

    #[test]
    fn search_finds_commands_by_intent() {
        for (query, expected) in [
            ("add to a dataset", "foxglove datasets episodes add"),
            ("create an episode from recordings", "foxglove episodes add"),
            ("download a dataset version", "foxglove datasets download"),
            ("upload a file", "foxglove upload"),
            ("find recordings for a device", "foxglove recordings list"),
            ("commit pending dataset changes", "foxglove datasets commit"),
            ("export mcap for a time range", "foxglove export"),
            ("log in", "foxglove auth login"),
        ] {
            let results = search(query);
            assert_eq!(
                results.first().map(String::as_str),
                Some(expected),
                "{query}: {results:?}"
            );
        }
    }

    #[test]
    fn search_accepts_unquoted_words_and_returns_empty_for_no_match() {
        assert_eq!(
            json(&["cli", "search", "add", "to", "a", "dataset"]),
            json(&["cli", "search", "add to a dataset"])
        );
        assert_eq!(json(&["cli", "search", "zzzz"]), Value::Array(Vec::new()));
    }

    #[test]
    fn describe_reports_arguments_options_and_usage() {
        let description = json(&["cli", "describe", "episodes", "add"]);
        assert_eq!(description["command"], "foxglove episodes add");
        assert_eq!(
            description["usage"],
            "foxglove episodes add [OPTIONS] --recording-id <RECORDING_ID>"
        );
        let options = description["options"].as_array().unwrap();
        let recording = options
            .iter()
            .find(|option| option["name"] == "--recording-id")
            .unwrap();
        assert_eq!(recording["required"], true);
        assert_eq!(recording["multiple"], true);
        assert_eq!(recording["type"], "string");
        assert_eq!(recording["global"], false);
        for name in ["--client-id", "--config", "--debug"] {
            let global = options
                .iter()
                .find(|option| option["name"] == name)
                .unwrap_or_else(|| panic!("{name} missing from {options:?}"));
            assert_eq!(global["global"], true, "{name}");
        }

        let download = json(&["cli", "describe", "datasets download"]);
        assert_eq!(download["arguments"][0]["name"], "DATASET_ID");
        assert_eq!(download["arguments"][0]["required"], true);
        let options = download["options"].as_array().unwrap();
        let attachments = options
            .iter()
            .find(|option| option["name"] == "--include-attachments")
            .unwrap();
        assert_eq!(attachments["type"], "boolean");
        assert_eq!(attachments["default"], true);
        assert_eq!(attachments["usage"], "--include-attachments[=true|false]");
        let version = options
            .iter()
            .find(|option| option["name"] == "--version")
            .unwrap();
        assert_eq!(version["type"], "integer");
        let output = options
            .iter()
            .find(|option| option["name"] == "--output")
            .unwrap();
        assert_eq!(output["short"], "-o");
    }

    #[test]
    fn describe_reports_enums_groups_and_subcommands() {
        let list = json(&["cli", "describe", "foxglove devices list"]);
        let format = list["options"]
            .as_array()
            .unwrap()
            .iter()
            .find(|option| option["name"] == "--format")
            .unwrap();
        assert_eq!(
            format["possibleValues"],
            serde_json::json!(["table", "json", "csv"])
        );
        assert_eq!(format["default"], "table");

        let edit = json(&["cli", "describe", "sessions", "edit"]);
        assert_eq!(
            edit["groups"],
            serde_json::json!([{"options": ["--key", "--remove-key"], "required": true, "multiple": false}])
        );

        let datasets = json(&["cli", "describe", "datasets"]);
        let children = datasets["subcommands"].as_array().unwrap();
        assert!(children
            .iter()
            .any(|child| child["command"] == "foxglove datasets episodes"));
        assert!(children
            .iter()
            .all(|child| child["command"] != "foxglove datasets help"));

        let root = json(&["cli", "describe"]);
        assert_eq!(root["command"], "foxglove");
        assert!(root["options"]
            .as_array()
            .unwrap()
            .iter()
            .any(|option| option["name"] == "--config" && option["global"] == true));
    }

    #[test]
    fn describe_rejects_unknown_commands_with_the_valid_choices() {
        let outcome = invoke(&["cli", "describe", "episodes", "remove"]);
        assert_eq!(outcome.exit_code, 1);
        assert!(outcome.stdout.is_empty());
        assert_eq!(
            String::from_utf8(outcome.stderr).unwrap(),
            "unknown command \"remove\"; `foxglove episodes` has: add, delete, get, list\n\
             Use `foxglove cli search <QUERY>` to find a command\n"
        );
    }

    #[test]
    fn every_executable_command_is_searchable_and_describable() {
        let mut root = crate::cli::command();
        root.build();
        let mut documents = Vec::new();
        super::collect_documents(&root, &mut Vec::new(), &mut Vec::new(), &mut documents);
        assert!(documents.len() > 40, "{}", documents.len());
        for document in &documents {
            assert!(!document.summary.is_empty(), "{}", document.command);
            let description = json(&["cli", "describe", &document.command]);
            assert_eq!(description["command"], document.command.as_str());
        }
    }
}
