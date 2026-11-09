//! Generate the repository's command reference without invoking the CLI.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use clap::Command;

const HEADER: &str =
    "<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->\n\n";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let check = match args.as_slice() {
        [] => false,
        [arg] if arg == "--check" => true,
        _ => return Err("usage: generate_docs [--check]".into()),
    };
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/reference");
    // Clap's color and terminal wrapping features are disabled in this crate,
    // so help rendering is independent of the terminal and environment.
    let mut command = foxglove_rust::command_definition().term_width(0);
    // Build the whole tree first so child pages include inherited global flags
    // and their complete invocation paths.
    command.build();
    let mut pages = BTreeMap::new();
    render(&command, &[], &mut pages);

    if !check {
        fs::create_dir_all(&directory)?;
    }
    let mut outdated = Vec::new();
    for (name, content) in &pages {
        let path = directory.join(name);
        if check {
            if fs::read_to_string(&path).ok().as_ref() != Some(content) {
                outdated.push(name.clone());
            }
        } else {
            fs::write(path, content)?;
        }
    }
    // The reference directory is generator-owned. Detect removed commands too,
    // including files that have not been added to Git yet.
    if directory.exists() {
        for entry in fs::read_dir(&directory)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if entry.path().extension().is_some_and(|ext| ext == "md") && !pages.contains_key(&name)
            {
                if check {
                    outdated.push(name);
                } else {
                    fs::remove_file(entry.path())?;
                }
            }
        }
    }
    if !outdated.is_empty() {
        outdated.sort();
        return Err(format!(
            "command reference is out of date: {}. Run `make docs`.",
            outdated.join(", ")
        )
        .into());
    }
    println!(
        "{} {} command reference pages",
        if check { "Checked" } else { "Generated" },
        pages.len()
    );
    Ok(())
}

fn filename(path: &[String]) -> String {
    if path.is_empty() {
        "README.md".to_owned()
    } else {
        format!("{}.md", path.join("-"))
    }
}

fn render(command: &Command, path: &[String], pages: &mut BTreeMap<String, String>) {
    let invocation = std::iter::once("foxglove")
        .chain(path.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(" ");
    let mut page = format!("{HEADER}# `{invocation}`\n\n");
    if path.is_empty() {
        page.push_str("[Documentation](../README.md) · [Getting started](../../README.md#getting-started)\n\nThis reference describes the commands in this checkout. For an installed release, use `foxglove <command> --help` to see its supported options.\n\n");
    } else {
        writeln!(
            page,
            "[Command reference](README.md) · [Parent command]({})\n",
            filename(&path[..path.len() - 1])
        )
        .unwrap();
    }
    let mut help = command.clone();
    let help = help.render_long_help().to_string();
    page.push_str("```text\n");
    for line in help.trim_end().lines() {
        writeln!(page, "{}", line.trim_end()).unwrap();
    }
    page.push_str("```\n");
    let children = command
        .get_subcommands()
        .filter(|child| !child.is_hide_set() && child.get_name() != "help")
        .collect::<Vec<_>>();
    if !children.is_empty() {
        page.push_str("\n## Subcommands\n\n");
        for child in &children {
            let mut child_path = path.to_vec();
            child_path.push(child.get_name().to_owned());
            writeln!(
                page,
                "- [`{invocation} {}`]({})",
                child.get_name(),
                filename(&child_path)
            )
            .unwrap();
            render(child, &child_path, pages);
        }
    }
    assert!(pages.insert(filename(path), page).is_none());
}
