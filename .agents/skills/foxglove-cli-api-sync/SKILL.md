---
name: foxglove-cli-api-sync
description: Update user-requested Foxglove Rust CLI commands or features against the public v1 OpenAPI spec, including scoped impact analysis and PR preparation.
---

# Foxglove CLI API sync

Sync only the commands, endpoints, or features the user requests. A spec revision, revision range, or API PR supplies source evidence; it does not authorize syncing every change it contains. If the requested scope cannot be inferred from the conversation, clarify it before implementation. Broader API coverage requires an explicit request.

## Establish scope and source

- Target `foxglove/foxglove-cli`; read its development instructions and Git status.
- Source: `foxglove/app`, [`packages/api/src/specs/v1.yaml`](https://github.com/foxglove/app/blob/main/packages/api/src/specs/v1.yaml). Use a supplied revision, or resolve current remote `main` to a full SHA and state that assumption. Retrieve committed content through authenticated GitHub access or `git show`; local uncommitted content is not upstream. Report inaccessible source rather than substituting docs.
- Use a supplied base when available. Otherwise compare only the requested CLI behavior with the pinned spec; no baseline is needed for a scoped update. A prior partial sync is not a repository-wide baseline.
- Honor the requested delivery mode: analysis, local changes, or PR. A request to analyze does not authorize implementation; a request to create a PR authorizes its commit, push, and creation.

## Determine and implement the requested changes

Read [references/repository-patterns.md](references/repository-patterns.md) before implementation for repository locations, compatibility constraints, and checks. Verify pointers against the current checkout.

Compare semantic contracts for the requested operations, identified by method and path. Follow shared `$ref` dependencies, composed schemas, and path-level parameters. Check relevant request serialization, omitted/null/empty values, response envelopes, pagination, timestamps, and output formats.

Trace shared-model effects to preserve other commands, but implement only changes necessary for the requested behavior. Do not add unrelated operations, fill historical gaps, or expand a revision-range request into full API coverage. Briefly flag incidental gaps only when useful.

Use [public API docs](https://docs.foxglove.dev/api) for usage context. A commit on main does not establish publication. If docs and spec materially disagree on requested behavior, report the discrepancy and avoid guessing; continue independent work.

Follow existing Rust command, Serde, client, error, and rendering patterns. Preserve established CLI behavior unless the requested change requires an adjustment; explain breaking changes. Update affected help and examples. Avoid unrelated refactors, SDK generation, or dependency upgrades.

Add focused regression tests for changed parsing, wire contracts, decoding, and output as relevant, using local fixture servers and isolated configuration. Run the repository checks in the reference and affected command help. Report failures or blocked checks accurately; ordinary `cargo test` omits the ignored HTTP tests.

## PR descriptions

Whenever drafting, creating, or updating a PR description, fetch and follow the [Foxglove PR template](https://github.com/foxglove/.github/blob/main/.github/pull_request_template.md), including for a local draft. Use authenticated GitHub access if needed. Preserve its section order and Before/After table:

- **Changelog:** Always include a concise, one-sentence user-facing changelog message. Use `None` only when there is no user-facing change.
- **Docs:** Link the documentation PR or tracking issue; use `None` when no documentation changes are needed. Do not invent links.
- **Description:** Explain the problem, resulting behavior, and relevant compatibility impact. Include validation performed and any blockers.
- **Before/After table:** Always include concrete invocations demonstrating each changed command, with relevant flags and expected behavior. For new commands, state that they were previously unavailable. Distinguish illustrative expected output from observed test results.

Include pinned spec permalinks and concise provenance in Description:
`API-Sync-Base: <full SHA or unknown>`,
`API-Sync-Head: <full SHA>`,
`API-Sync-Scope: <requested operations/features actually covered>`.
Record deferred requested work and publication uncertainty when relevant. Keep the description proportional to the change. If the template cannot be fetched, disclose that limitation and use the structure above.

## Deliver

When a PR is requested, check for existing work covering the same source and scope before creating it. Use `guida/api-sync-<short-spec-sha>-<scope>` unless a branch is supplied. Stage only intended files; do not create an empty PR. For `gh`, use `--body-file` and explicit repository/base/head options.

Create a ready PR when checks pass and semantics are resolved; use a draft with explicit blockers otherwise. Verify remote state before retrying an uncertain push or PR creation. Do not merge, release, or modify the source API as part of a sync.

Return the PR link or local result, source revision, concise behavior changes, validation, and any unfinished requested work.
