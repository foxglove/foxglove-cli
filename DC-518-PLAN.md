# DC-518: Session support for filtering recordings and data operations

Sessions work like recordings: optional start/end, identified by **session-id** or **session-key**, with the rule that **session-key always requires project-id**.

This document lists all code touchpoints and an ordered implementation plan.

---

## 1. Discovery: Code touchpoints

### 1.1 API types and validation (`foxglove/api/api.go`)

| Area | Current recording behavior | Session changes |
|------|----------------------------|------------------|
| **StreamRequest** (lines 37–59) | `RecordingID`, `Key`; no `ProjectID`; `Validate()` requires one of recording-id/key, import-id, or device+start/end | Add `SessionID`, `SessionKey`; add `ProjectID` (or session-scoped project field) for session-key. Extend `Validate()`: allow session-id or session-key; if session-key then require project-id. |
| **AttachmentsRequest** (116–119) | `ImportID`, `RecordingID` | Add `SessionID`, `SessionKey`; when using session-key, API (and CLI) will need project-id. |
| **CoverageRequest** (391–401) | `RecordingID`, `ProjectID`, device, start/end | Add `SessionID`, `SessionKey`; session-key implies project-id. |
| **RecordingsRequest** (194–206) | `ProjectID`, device, start/end, path, site, etc. (no recording id/key) | Add `SessionID`, `SessionKey` so “list recordings” can be filtered by session; session-key requires project-id. |
| **UploadRequest** (25–30) | `ProjectID`, `DeviceID`, `DeviceName`, `Key` (recording key) | If uploads can target a session: add `SessionID`, `SessionKey` and require project-id when using session-key. |
| **PendingImportsRequest** (531–544) | `Key`, `ProjectID`, device, etc. | If backend supports filtering by session: add `SessionID`, `SessionKey`; session-key + project-id. |

**api.go** is the single source of truth for request structs and shared validation (e.g. session-key ⇒ project-id).

### 1.2 API client (`foxglove/api/client.go`)

- **Stream**, **Attachments**, **Coverage**, **Recordings**, **PendingImports**, **Upload** all take request structs. Once those structs have session fields (and project-id where needed), the client just serializes them—no new client methods unless the backend adds dedicated session endpoints (e.g. `GET /sessions/:id`).
- If new session endpoints exist, add corresponding client methods (e.g. `Session(sessionID string)` or `Sessions(req *SessionsRequest)`).

### 1.3 API helpers (`foxglove/api/lib.go`)

- **Export** – takes `*StreamRequest`; no change except callers passing session fields.
- **Import** – takes `projectID`, `deviceID`, `deviceName`, `key`. If uploads can target a session, add session params and pass them into `UploadRequest` (and possibly extend the function signature for session-key + project-id).

### 1.4 CLI commands that filter or target by recording (and should support session)

| Command | File | Recording usage | Session changes |
|---------|------|-----------------|-----------------|
| **data export** | `foxglove/cmd/export.go` | `--recording-id`, `--key`; `createStreamRequest(recordingID, key, ...)` builds `StreamRequest` | Add `--session-id`, `--session-key`; require `--project-id` when `--session-key` is set. Extend `createStreamRequest` (or equivalent) to accept session args and set `SessionID`/`SessionKey`/`ProjectID` on `StreamRequest`. |
| **data import** | `foxglove/cmd/import.go` | `--project-id`, `--device-id`, `--device-name`, `--key`; `executeImport(..., projectID, deviceID, deviceName, key, ...)` | If uploads can be to a session: add `--session-id`, `--session-key`, require `--project-id` for session-key; pass session params into `executeImport` and `UploadRequest`. |
| **attachments list** | `foxglove/cmd/attachments.go` | `--recording-id`, `--import-id` → `AttachmentsRequest` | Add `--session-id`, `--session-key`; require `--project-id` when `--session-key`; pass into `AttachmentsRequest`. |
| **coverage list** | `foxglove/cmd/coverage.go` | `--recording-id`, `--project-id`, device, start/end → `CoverageRequest` | Add `--session-id`, `--session-key`; require `--project-id` when `--session-key`; pass into `CoverageRequest`. |
| **recordings list** | `foxglove/cmd/recordings.go` | `RecordingsRequest` with project, device, start/end (no recording id/key today) | Add `--session-id`, `--session-key`; require `--project-id` when `--session-key`; pass into `RecordingsRequest`. |
| **pending-imports list** | `foxglove/cmd/pending_imports.go` | `Key`, `ProjectID`, device, etc. → `PendingImportsRequest` | If API supports session filter: add `--session-id`, `--session-key`; require `--project-id` for session-key; pass into `PendingImportsRequest`. |

### 1.5 Root / command tree (`foxglove/cmd/root.go`)

- No change unless you add a dedicated **sessions** command (e.g. `foxglove sessions list`, `foxglove sessions get`). Then add a `sessionsCmd` and subcommands that call new or existing client methods.

### 1.6 Tests and mocks

- **foxglove/api/api_test.go** – Recording-related tests; add or extend tests for `StreamRequest.Validate()` (and any new validation) with session-id / session-key + project-id.
- **foxglove/api/lib_test.go** – Export with `StreamRequest`; add cases that use session fields.
- **foxglove/cmd/export_test.go** – `createStreamRequest` and flag parsing; add cases for session-id, session-key, and project-id requirement when session-key is set.
- **foxglove/api/mock_service.go** – `stream` (and any other handlers that take request bodies): if you want integration tests to use sessions, handle `SessionID`/`SessionKey` (and project-id) in the mock.

### 1.7 Shared “key + project” rule

- Today there’s no central validation that “recording key ⇒ project-id”. For sessions you want a single rule: **session-key ⇒ project-id required**.
- Options:
  - Implement validation in **api.go** (e.g. `StreamRequest.Validate()` and any other request validators that accept session-key).
  - Optionally add a small helper in **cmd** (e.g. `requireProjectIDForSessionKey(sessionKey, projectID)`) and use it in each command that accepts `--session-key`, so CLI and API stay consistent.

---

## 2. Implementation plan (ordered)

1. **API types and validation (api.go)**  
   Add `SessionID`, `SessionKey` (and where needed `ProjectID`) to: `StreamRequest`, `AttachmentsRequest`, `CoverageRequest`, `RecordingsRequest`; optionally `UploadRequest`, `PendingImportsRequest`.  
   In `StreamRequest.Validate()` (and any other relevant validators): allow session-id or session-key; if session-key is set, require project-id.  
   Keep recording-id/key behavior unchanged; add session behavior alongside it.

2. **CLI validation helper (optional but recommended)**  
   In `foxglove/cmd` (e.g. a small util or in a shared command helper): e.g. `requireProjectIDForSessionKey(sessionKey, projectID)` that returns an error if sessionKey != "" && projectID == "". Use it in every command that has `--session-key`.

3. **Export (cmd/export.go)**  
   Add flags `--session-id`, `--session-key`; ensure `--project-id` is used when session-key is set.  
   Extend `createStreamRequest` to take session (and project) args and set `SessionID`, `SessionKey`, `ProjectID` on `StreamRequest`.  
   Run validation (and helper if added) in `Run`.

4. **Import (cmd/import.go)**  
   If uploads can target a session: add `--session-id`, `--session-key`, require project-id for session-key; pass session params through `executeImport` into `UploadRequest`.

5. **Attachments list (cmd/attachments.go)**  
   Add `--session-id`, `--session-key`, require project-id for session-key; pass into `AttachmentsRequest`.

6. **Coverage list (cmd/coverage.go)**  
   Add `--session-id`, `--session-key`, require project-id for session-key; pass into `CoverageRequest`.

7. **Recordings list (cmd/recordings.go)**  
   Add `--session-id`, `--session-key`, require project-id for session-key; pass into `RecordingsRequest`.

8. **Pending imports list (cmd/pending_imports.go)**  
   Only if the backend supports filtering by session: add `--session-id`, `--session-key`, require project-id for session-key; pass into `PendingImportsRequest`.

9. **API client (client.go)**  
   Add new session endpoints only if the backend exposes them; otherwise no changes.

10. **Lib (lib.go)**  
    If import can target a session: extend `Import` (and any callers) with session params and set them on `UploadRequest`.

11. **Tests**  
    api: validate session validation rules and request serialization.  
    cmd/export: `createStreamRequest` and flags for session-id, session-key, project-id.  
    lib: Export with session in `StreamRequest`.  
    mock_service: handle session in `stream` (and any other handlers that take session fields) if you want session-based integration tests.

12. **Root (root.go)**  
    Only if you add a `sessions` command: add `sessionsCmd` and subcommands.

---

## 3. Summary table

| Location | Purpose |
|----------|--------|
| **api/api.go** | Add session fields and “session-key ⇒ project-id” validation to Stream, Attachments, Coverage, Recordings; optionally Upload, PendingImports. |
| **api/client.go** | New methods only if backend has session-specific endpoints. |
| **api/lib.go** | Pass session params into Import/Upload if uploads can target a session. |
| **cmd/export.go** | Session flags + createStreamRequest + validation. |
| **cmd/import.go** | Session flags + executeImport/UploadRequest if upload supports session. |
| **cmd/attachments.go** | Session flags + AttachmentsRequest. |
| **cmd/coverage.go** | Session flags + CoverageRequest. |
| **cmd/recordings.go** | Session flags + RecordingsRequest. |
| **cmd/pending_imports.go** | Session flags + PendingImportsRequest if API supports it. |
| **cmd (shared)** | Optional helper for “session-key ⇒ project-id” in CLI. |
| **Tests + mock** | Validation, createStreamRequest, Export, and mock handlers for session. |
| **cmd/root.go** | Only if adding a `sessions` top-level command. |
