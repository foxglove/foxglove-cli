# DC-518: OpenAPI spec review (session support)

Review of the Foxglove OpenAPI spec against the DC-518 session feature. Rule: **session-key always requires project-id**.

---

## 1. Missing in spec

### 1.1 Recording Attachments – no session filter

**Path:** `GET /recording-attachments`

**Current:** Parameters include `recordingId`, `importId`, `deviceId`, `deviceName`, `projectId`. No session parameters.

**Gap:** Per DC-518, attachments list should be filterable by session. The spec does not define:

- `sessionId` (optional) – filter by session ID
- `sessionKey` (optional) – filter by session key

**Recommendation:** Add `sessionId` and `sessionKey` query parameters. Document that **`projectId` is required when `sessionKey` is supplied**.

---

### 1.2 Pending Imports – no session filter (if supported)

**Path:** `GET /data/pending-imports`

**Current:** Parameters include `key`, `projectId`, `deviceId`, `deviceName`, etc. No session parameters.

**Gap:** If the backend supports filtering pending imports by session, the spec does not expose it.

**Recommendation:** If the API supports it: add `sessionId` and `sessionKey`, and state that **`projectId` is required when `sessionKey` is supplied**.

---

### 1.3 Upload – no session targeting (if supported)

**Path:** `POST /data/upload`

**Current:** Body has `filename`, `deviceId`, `deviceName`, `key`, `projectId`. No session fields.

**Gap:** If uploads can target a session (e.g. “add this file to session X”), the spec does not define session input.

**Recommendation:** If the API supports it: add optional `sessionId` and `sessionKey` to the request body, and document that **`projectId` is required when `sessionKey` is supplied**.

---

### 1.4 Sessions tag and tag group

**Current:** Session endpoints use `tags: [Sessions]`, but:

- There is **no `Sessions` tag definition** in the top-level `tags` array (unlike Devices, Recordings, etc.).
- **`x-tagGroups`** under "Data Management" does not list "Sessions", so Sessions won’t appear in the doc navigation.

**Recommendation:**

- Add a tag definition, e.g.:

  ```yaml
  - name: Sessions
    description: |
      Recording sessions group recordings (e.g. from a device). You can filter and operate on data by session using session ID or session key; when using session key, project ID is required.
  ```

- Add `Sessions` to the Data Management group in `x-tagGroups`.

---

## 2. Documentation / clarity gaps

### 2.1 Stream (download) – description omits sessions

**Path:** `POST /data/stream`

**Current:** Description says: “One of `recordingId`, `key`, `importId` (deprecated) or all three of `deviceId`/`deviceName`, `start`, and `end` must be specified.”

**Gap:** It does not mention `sessionId` / `sessionKey` as valid ways to specify the data source, even though the request body defines them.

**Recommendation:** Extend the description to include: “Alternatively, one of `sessionId` or `sessionKey` may be specified; when using `sessionKey`, `projectId` is required.”

---

### 2.2 Recordings list – projectId when using sessionKey

**Path:** `GET /recordings`

**Current:** Has `sessionId` and `sessionKey` (x-internal). `projectId` exists but is not tied to session-key usage.

**Gap:** The rule “session-key ⇒ project-id required” is not stated for this endpoint.

**Recommendation:** In the parameter description for `projectId` and/or `sessionKey`, state that **`projectId` is required when filtering by `sessionKey`**.

---

### 2.3 Coverage – projectId when using sessionKey; start/end optional?

**Path:** `GET /data/coverage`

**Current:** Has `sessionId`, `sessionKey` (x-internal), and `projectId`. Description says “You must specify the `start` and `end` arguments.”

**Gaps:**

- It is not documented that **`projectId` is required when using `sessionKey`**.
- When filtering by `recordingId` or by session, implementations often allow omitting `start`/`end`. The spec does not say when `start`/`end` are optional.

**Recommendation:**

- Document that **`projectId` is required when `sessionKey` is supplied**.
- If the API allows coverage by recording or session without start/end, clarify that **`start` and `end` are optional when `recordingId`, `sessionId`, or `sessionKey` is provided** (or words to that effect).

---

### 2.4 Topics – projectId when using sessionKey

**Path:** `GET /data/topics`

**Current:** Has `sessionId`, `sessionKey` (x-internal), and `projectId`.

**Gap:** The rule “session-key ⇒ project-id required” is not stated.

**Recommendation:** Document that **`projectId` is required when `sessionKey` is supplied**.

---

### 2.5 Lake files – projectId when using sessionKey

**Path:** `GET /lake-files`

**Current:** Has `sessionId` and `sessionKey` (x-internal). No `projectId` in the listed parameters.

**Gap:** If the backend requires `projectId` when using `sessionKey` for lake files, the spec does not define or describe it.

**Recommendation:** If the API requires it: add a `projectId` query parameter and document that **`projectId` is required when `sessionKey` is supplied**.

---

### 2.6 GET /sessions/{keyOrId} – projectId description

**Path:** `GET /sessions/{keyOrId}`

**Current:** `projectId` is required; description is “Filter sessions by project.”

**Gap:** When `keyOrId` is a **session key**, `projectId` is required to resolve the session, not just to “filter.” The description is a bit misleading.

**Recommendation:** Clarify, e.g.: “Project ID that the session belongs to. Required when `keyOrId` is a session key (to resolve the session).”

---

## 3. Already aligned with DC-518

- **POST /data/stream** – Request body includes `sessionId`, `sessionKey`, and `projectId` with description “Required if streaming by sessionKey.”
- **GET /recordings** – Has `sessionId` and `sessionKey` (x-internal).
- **GET /data/coverage** – Has `sessionId`, `sessionKey`, and `projectId`.
- **GET /data/topics** – Has `sessionId`, `sessionKey`, and `projectId`.
- **GET /lake-files** – Has `sessionId` and `sessionKey`.
- **GET/PATCH/DELETE /sessions/{keyOrId}** – `projectId` is required when using keyOrId (session key).

---

## 4. Summary table

| Item | Action |
|------|--------|
| **GET /recording-attachments** | Add `sessionId`, `sessionKey`; document projectId required for sessionKey. |
| **GET /data/pending-imports** | If supported: add `sessionId`, `sessionKey`; document projectId required for sessionKey. |
| **POST /data/upload** | If supported: add `sessionId`, `sessionKey`; document projectId required for sessionKey. |
| **Tags** | Define `Sessions` tag; add `Sessions` to Data Management in `x-tagGroups`. |
| **POST /data/stream** | Update description to mention sessionId/sessionKey and projectId for sessionKey. |
| **GET /recordings** | Document projectId required when using sessionKey. |
| **GET /data/coverage** | Document projectId required for sessionKey; clarify when start/end are optional. |
| **GET /data/topics** | Document projectId required when using sessionKey. |
| **GET /lake-files** | If applicable: add/document projectId and “required when sessionKey” rule. |
| **GET /sessions/{keyOrId}** | Clarify projectId description for session-key resolution. |

---

## 5. Optional: response shapes

If recordings (or other resources) can belong to a session, consider whether response schemas (e.g. `Recording`) should include session fields (e.g. `sessionId` or `session`) so clients can display or filter by session without an extra round-trip. This depends on the actual API responses and is optional for the spec review.
