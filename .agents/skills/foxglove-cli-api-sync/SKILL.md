---
name: foxglove-cli-api-sync
description: Update user-requested Foxglove Rust CLI commands or features against the public v1 OpenAPI spec, including scoped impact analysis and PR preparation.
---

# Foxglove CLI API sync

Sync only the commands, endpoints, or features the user requests. A spec revision, revision range, or API PR supplies source evidence; it does not authorize syncing every change it contains. If the requested scope cannot be inferred from the conversation, clarify it before implementation. Broader API coverage requires an explicit request.

## Public repository content

This repository is public. Keep PR descriptions, comments, commit messages, documentation, code, and test fixtures suitable for public readers. Explain public API and CLI behavior without disclosing internal implementation details, private repository paths, revisions, PRs, issues, or links. Use only publicly accessible references in repository content. Never mention open security issues in this repository, including their existence or details.

## Establish scope and source

- Target `foxglove/foxglove-cli`; read its development instructions and Git status.
- Source: the public v1 OpenAPI contract. When a source repository is supplied in the conversation, use a supplied revision, or resolve its current remote `main` to a full SHA and state that assumption in the private conversation. Retrieve committed content through authenticated GitHub access or `git show`; local uncommitted content is not upstream. Report inaccessible source rather than substituting docs.
- Use a supplied base when available. Otherwise compare only the requested CLI behavior with the pinned spec; no baseline is needed for a scoped update. A prior partial sync is not a repository-wide baseline.
- An implementation ticket is a tracker issue supplied to this skill as work to implement. Questions asking for analysis or recommendations do not trigger implementation or PR creation.
- Honor explicit delivery modes: analysis, local changes, or PR. When this skill is invoked with an implementation ticket and no delivery mode, implement, self-review, validate, and open a PR without a follow-up request. An analysis-only or local-only request overrides this default.

## Determine and implement the requested changes

Read [references/repository-patterns.md](references/repository-patterns.md) before implementation for repository locations, compatibility constraints, and checks. Verify pointers against the current checkout.

Compare semantic contracts for the requested operations, identified by method and path. Follow shared `$ref` dependencies, composed schemas, and path-level parameters. Check relevant request serialization, omitted/null/empty values, response envelopes, pagination, timestamps, and output formats.

Trace shared-model effects to preserve other commands, but implement only changes necessary for the requested behavior. Do not add unrelated operations, fill historical gaps, or expand a revision-range request into full API coverage. Briefly flag incidental gaps only when useful.

Use [public API docs](https://docs.foxglove.dev/api) for usage context. A commit on main does not establish publication. If docs and spec materially disagree on requested behavior, report the discrepancy and avoid guessing; continue independent work.

Follow existing Rust command, Serde, client, error, and rendering patterns. Preserve established CLI behavior unless the requested change requires an adjustment; explain breaking changes. Update affected help and examples. Avoid unrelated refactors, SDK generation, or dependency upgrades.

Extend an existing focused test before adding a new test or fixture. For a response-model field handled entirely by Serde, extend the existing serialization/deserialization test to cover the field and an older response that omits it; no new CLI integration test is needed unless request routing, HTTP handling, envelopes, or rendering logic also changes. Test distinct behavior rather than every string value when the model merely preserves strings. Use local fixture servers and isolated configuration when HTTP coverage is warranted. Run the repository checks in the reference. Run affected command help when the command tree changes. Report failures or blocked checks accurately; ordinary `cargo test` omits the ignored HTTP tests.

Before delivery, review the diff for correctness and simplicity. Remove redundant tests, fixtures, abstractions, and unrelated changes. For a shared-model addition, make the smallest change that satisfies the requested output behavior. Update `Record::headers()` and `Record::fields()` when the field belongs in CSV/table output; preserve existing summaries for nested JSON-only metadata. Extend existing coverage for the affected formats.

## PR descriptions

Whenever drafting, creating, or updating a PR description, fetch and follow the [Foxglove PR template](https://github.com/foxglove/.github/blob/main/.github/pull_request_template.md), including for a local draft. Use authenticated GitHub access if needed. Preserve its section order and Before/After table:

- **Changelog:** Always include a concise, one-sentence user-facing changelog message. Use `None` only when there is no user-facing change.
- **Docs:** Link the documentation PR or tracking issue; use `None` when no documentation changes are needed. Do not invent links.
- **Description:** For a small change, use one short paragraph explaining the problem, resulting behavior, and compatibility, plus one concise validation sentence. Mention material blockers or publication uncertainty briefly; omit process history and repeated implementation details.
- **Before/After table:** Keep the template table, but use one representative invocation for a shared-model change; briefly name other affected commands in prose. Give separate examples only when command behavior differs. Include relevant flags and expected behavior. For new commands, state that they were previously unavailable. Distinguish illustrative expected output from observed test results.

Include pinned spec permalinks in Description when useful to explain the change.
Record deferred requested work and publication uncertainty when relevant. Keep the description proportional to the change. If the template cannot be fetched, disclose that limitation and use the structure above.

## Deliver

When PR creation is the delivery mode, whether explicitly requested or selected by default, check for existing work covering the same source and scope before creating it. Use `<user>/api-sync-<short-spec-sha>-<scope>` unless a branch is supplied. Stage only intended files; do not create an empty PR. For `gh`, use `--body-file` and explicit repository/base/head options.

Create a ready PR when checks pass and semantics are resolved; use a draft with explicit blockers otherwise. Verify remote state before retrying an uncertain push or PR creation. Do not merge, release, or modify the source API as part of a sync.

Return the PR link or local result, source revision, concise behavior changes, validation, and any unfinished requested work.
