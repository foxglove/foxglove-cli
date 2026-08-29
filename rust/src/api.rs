//! Asynchronous Foxglove Data Platform API client.
//!
//! The client deliberately keeps the HTTP and transfer concerns separate from
//! command execution. Commands can therefore stage output and decide their own
//! cleanup policy while this module guarantees that response bodies are either
//! consumed or dropped on every branch.

use std::fmt;
use std::io;
use std::sync::{Arc, RwLock};

use reqwest::{Method, RequestBuilder, Response, StatusCode, Url};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio_util::io::ReaderStream;
use tokio_util::sync::CancellationToken;

const FORBIDDEN_MESSAGE: &str = "forbidden: have you signed in with `foxglove auth login`?";

/// Errors returned by the API client.
#[derive(Debug)]
pub enum ApiError {
    /// The API rejected the credentials with HTTP 401 or 403.
    Forbidden,
    /// The requested resource does not exist.
    NotFound,
    /// The server returned a non-success response with its decoded message.
    Response { status: u16, message: String },
    /// The request could not be sent or its response could not be read.
    Transport(reqwest::Error),
    /// A response was syntactically valid HTTP but not valid expected JSON.
    Decode(reqwest::Error),
    /// A local request or response value could not be serialized.
    Serialization(serde_json::Error),
    /// The operation was cancelled before it completed.
    Cancelled,
    /// A base URL or redirect URL was invalid.
    InvalidUrl(String),
    /// A streamed destination rejected a chunk.
    Write(io::Error),
}

impl ApiError {
    /// Whether the server rejected the current authentication.
    #[must_use]
    pub const fn is_forbidden(&self) -> bool {
        matches!(self, Self::Forbidden)
    }

    /// Whether the requested resource was not found.
    #[must_use]
    pub const fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound)
    }

    /// Whether this operation was cancelled by its caller.
    #[must_use]
    pub const fn is_cancelled(&self) -> bool {
        matches!(self, Self::Cancelled)
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Forbidden => formatter.write_str(FORBIDDEN_MESSAGE),
            Self::NotFound => formatter.write_str("not found"),
            Self::Response { message, .. } => formatter.write_str(message),
            Self::Transport(error) | Self::Decode(error) => error.fmt(formatter),
            Self::Serialization(error) => error.fmt(formatter),
            Self::Cancelled => formatter.write_str("operation cancelled"),
            Self::InvalidUrl(error) => formatter.write_str(error),
            Self::Write(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ApiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Transport(error) | Self::Decode(error) => Some(error),
            Self::Serialization(error) => Some(error),
            Self::Write(error) => Some(error),
            Self::Forbidden
            | Self::NotFound
            | Self::Response { .. }
            | Self::Cancelled
            | Self::InvalidUrl(_) => None,
        }
    }
}

/// The request accepted by `/v1/data/stream`.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamRequest {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub recording_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub key: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub import_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub project_id: String,
    #[serde(rename = "device.id", skip_serializing_if = "String::is_empty")]
    pub device_id: String,
    #[serde(rename = "device.name", skip_serializing_if = "String::is_empty")]
    pub device_name: String,
    #[serde(
        with = "time::serde::rfc3339::option",
        skip_serializing_if = "Option::is_none"
    )]
    pub start: Option<OffsetDateTime>,
    #[serde(
        with = "time::serde::rfc3339::option",
        skip_serializing_if = "Option::is_none"
    )]
    pub end: Option<OffsetDateTime>,
    pub output_format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression_format: Option<String>,
    #[serde(skip_serializing_if = "is_false")]
    pub include_attachments: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub replay_policy: String,
    #[serde(skip_serializing_if = "is_zero")]
    pub replay_lookback_seconds: f64,
    pub topics: Vec<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub session_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub session_key: String,
}

impl StreamRequest {
    /// Validate source selection and output-specific options before a request
    /// is sent, matching the Go client's command-level contract.
    ///
    /// # Errors
    ///
    /// Returns a compatibility error when the source or output options are
    /// incomplete or contradictory.
    pub fn validate(&self) -> Result<(), String> {
        let recording = !self.recording_id.is_empty() || !self.key.is_empty();
        let session = !self.session_id.is_empty() || !self.session_key.is_empty();
        let device = !self.device_id.is_empty() || !self.device_name.is_empty();
        let import = !self.import_id.is_empty();
        if !(recording || session || device || import) {
            return Err("either recording-id/key, session-id/session-key, import-id, or device-id/device-name with start/end are required".to_owned());
        }
        if !self.session_key.is_empty() && self.project_id.is_empty() {
            return Err("project-id is required when using session-key".to_owned());
        }
        if device
            && !import
            && !recording
            && !session
            && (self.start.is_none() || self.end.is_none())
        {
            return Err(
                "start/end are required if device is supplied without recording or session"
                    .to_owned(),
            );
        }
        if let (Some(start), Some(end)) = (self.start, self.end) {
            if end < start {
                return Err("end must be after or equal to start".to_owned());
            }
        }
        if let Some(compression) = &self.compression_format {
            if !matches!(compression.as_str(), "" | "zstd" | "lz4") {
                return Err(format!(
                    "invalid compression format {compression:?}: supply \"\", zstd, or lz4"
                ));
            }
            if !is_mcap_format(&self.output_format) {
                return Err("compression format is only valid for mcap output".to_owned());
            }
        }
        if self.include_attachments && !is_mcap_format(&self.output_format) {
            return Err("include-attachments is only valid for mcap output".to_owned());
        }
        if !self.replay_policy.is_empty() && self.replay_policy != "lastPerChannel" {
            return Err(format!(
                "invalid replay policy {:?}: supply \"\" or lastPerChannel",
                self.replay_policy
            ));
        }
        if self.replay_lookback_seconds < 0.0 {
            return Err("replay-lookback-seconds must be >= 0".to_owned());
        }
        Ok(())
    }
}

/// The link returned before a stream is downloaded.
#[derive(Clone, Debug, Deserialize)]
pub struct StreamResponse {
    pub link: String,
}

/// The signed upload link returned by the import API.
#[derive(Clone, Debug, Deserialize)]
pub struct UploadResponse {
    pub link: String,
}

/// Metadata needed by the upload redirect request.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadRequest {
    pub filename: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub project_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub key: String,
    #[serde(rename = "device.id", skip_serializing_if = "String::is_empty")]
    pub device_id: String,
    #[serde(rename = "device.name", skip_serializing_if = "String::is_empty")]
    pub device_name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub session_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub session_key: String,
}

/// Device-code response used by the interactive login flow.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub expires_in: u64,
    pub interval: u64,
    pub verification_uri: String,
    pub verification_uri_complete: String,
}

/// The authenticated API client.
#[derive(Clone, Debug)]
pub struct FoxgloveClient {
    http: reqwest::Client,
    base_url: Url,
    client_id: String,
    user_agent: String,
    token: Arc<RwLock<String>>,
}

/// Install a portable Ctrl-C listener and return the token it cancels.
///
/// Callers pass the returned token to one of the `*_with_cancellation`
/// methods. The listener is intentionally kept outside command execution so
/// a command can clean up staging files before deciding how to report exit
/// status.
#[must_use]
pub fn ctrl_c_cancellation_token() -> CancellationToken {
    let cancellation = CancellationToken::new();
    let listener_token = cancellation.clone();
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                listener_token.cancel();
            }
        });
    }
    cancellation
}

impl FoxgloveClient {
    /// Build a client backed by the remote API.
    ///
    /// # Errors
    ///
    /// Returns an error when the base URL is invalid or the HTTP client cannot
    /// be initialized.
    pub fn new(
        base_url: &str,
        client_id: impl Into<String>,
        token: impl Into<String>,
        user_agent: impl Into<String>,
    ) -> Result<Self, ApiError> {
        let mut base_url = Url::parse(base_url)
            .map_err(|error| ApiError::InvalidUrl(format!("invalid API base URL: {error}")))?;
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        Ok(Self {
            http: reqwest::Client::builder()
                .build()
                .map_err(ApiError::Transport)?,
            base_url,
            client_id: client_id.into(),
            user_agent: user_agent.into(),
            token: Arc::new(RwLock::new(token.into())),
        })
    }

    /// The client ID used for device-code authentication.
    #[must_use]
    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    /// Replace the bearer token used for authenticated requests.
    pub fn set_token(&self, token: impl Into<String>) {
        if let Ok(mut current) = self.token.write() {
            *current = token.into();
        }
    }

    /// Sign an ID token in and store the returned bearer token on this client.
    ///
    /// # Errors
    ///
    /// Returns the mapped API, transport, or response-decoding error.
    pub async fn sign_in(&self, id_token: &str) -> Result<String, ApiError> {
        #[derive(Serialize)]
        struct SignInRequest<'a> {
            #[serde(rename = "idToken")]
            token: &'a str,
        }
        #[derive(Deserialize)]
        struct SignInResponse {
            #[serde(rename = "bearerToken")]
            token: String,
        }
        let response: SignInResponse = self
            .send_json(
                Method::POST,
                "/v1/signin",
                &SignInRequest { token: id_token },
                false,
                None,
            )
            .await?;
        self.set_token(response.token.clone());
        Ok(response.token)
    }

    /// Fetch a device code for the interactive login flow.
    ///
    /// # Errors
    ///
    /// Returns the mapped API, transport, or response-decoding error.
    pub async fn device_code(&self) -> Result<DeviceCodeResponse, ApiError> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct DeviceCodeRequest<'a> {
            client_id: &'a str,
        }
        self.send_json(
            Method::POST,
            "/v1/auth/device-code",
            &DeviceCodeRequest {
                client_id: &self.client_id,
            },
            false,
            None,
        )
        .await
    }

    /// Poll for the ID token associated with a device code.
    ///
    /// # Errors
    ///
    /// Returns the mapped API, transport, or response-decoding error.
    pub async fn token(&self, device_code: &str) -> Result<String, ApiError> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct TokenRequest<'a> {
            client_id: &'a str,
            device_code: &'a str,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct TokenResponse {
            id_token: String,
        }
        let response: TokenResponse = self
            .send_json(
                Method::POST,
                "/v1/auth/token",
                &TokenRequest {
                    client_id: &self.client_id,
                    device_code,
                },
                false,
                None,
            )
            .await?;
        Ok(response.id_token)
    }

    /// Execute an authenticated GET and decode its JSON response.
    ///
    /// # Errors
    ///
    /// Returns the mapped API, transport, or response-decoding error.
    pub async fn get<Q, T>(&self, endpoint: &str, query: &Q) -> Result<T, ApiError>
    where
        Q: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        self.get_with_cancellation(endpoint, query, &CancellationToken::new())
            .await
    }

    /// Execute an authenticated GET with cancellation support.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::Cancelled`] when cancellation wins, or the mapped
    /// API, transport, or response-decoding error.
    pub async fn get_with_cancellation<Q, T>(
        &self,
        endpoint: &str,
        query: &Q,
        cancellation: &CancellationToken,
    ) -> Result<T, ApiError>
    where
        Q: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        self.send_json_with_query(
            Method::GET,
            endpoint,
            query,
            None::<&()>,
            Some(cancellation),
        )
        .await
    }

    /// Execute an authenticated JSON POST and decode its JSON response.
    ///
    /// # Errors
    ///
    /// Returns the mapped API, transport, or response-decoding error.
    pub async fn post<B, T>(&self, endpoint: &str, body: &B) -> Result<T, ApiError>
    where
        B: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        self.post_with_cancellation(endpoint, body, &CancellationToken::new())
            .await
    }

    /// Execute an authenticated JSON POST with cancellation support.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::Cancelled`] when cancellation wins, or the mapped
    /// API, transport, or response-decoding error.
    pub async fn post_with_cancellation<B, T>(
        &self,
        endpoint: &str,
        body: &B,
        cancellation: &CancellationToken,
    ) -> Result<T, ApiError>
    where
        B: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        self.send_json(Method::POST, endpoint, body, true, Some(cancellation))
            .await
    }

    /// Execute an authenticated JSON PATCH and decode its JSON response.
    ///
    /// # Errors
    ///
    /// Returns the mapped API, transport, or response-decoding error.
    pub async fn patch<Q, B, T>(&self, endpoint: &str, query: &Q, body: &B) -> Result<T, ApiError>
    where
        Q: Serialize + ?Sized,
        B: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        self.patch_with_cancellation(endpoint, query, body, &CancellationToken::new())
            .await
    }

    /// Execute an authenticated JSON PATCH with cancellation support.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::Cancelled`] when cancellation wins, or the mapped
    /// API, transport, or response-decoding error.
    pub async fn patch_with_cancellation<Q, B, T>(
        &self,
        endpoint: &str,
        query: &Q,
        body: &B,
        cancellation: &CancellationToken,
    ) -> Result<T, ApiError>
    where
        Q: Serialize + ?Sized,
        B: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        self.send_json_with_query(
            Method::PATCH,
            endpoint,
            query,
            Some(body),
            Some(cancellation),
        )
        .await
    }

    /// Execute an authenticated DELETE and consume its response body.
    ///
    /// # Errors
    ///
    /// Returns the mapped API or transport error.
    pub async fn delete(&self, endpoint: &str) -> Result<(), ApiError> {
        self.delete_with_cancellation(endpoint, &CancellationToken::new())
            .await
    }

    /// Execute an authenticated DELETE with cancellation support.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::Cancelled`] when cancellation wins, or the mapped
    /// API or transport error.
    pub async fn delete_with_cancellation(
        &self,
        endpoint: &str,
        cancellation: &CancellationToken,
    ) -> Result<(), ApiError> {
        let response =
            send_with_cancellation(self.request(Method::DELETE, endpoint)?, cancellation).await?;
        ensure_success(response).await
    }

    /// Request a streamed data download.
    ///
    /// # Errors
    ///
    /// Returns the mapped API, transport, URL, or response-decoding error.
    pub async fn stream(&self, request: &StreamRequest) -> Result<ResponseStream, ApiError> {
        self.stream_with_cancellation(request, &CancellationToken::new())
            .await
    }

    /// Request a streamed data download, cancelling both the link request and
    /// the active download when the supplied token is cancelled.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::Cancelled`] when cancellation wins, or the mapped
    /// API, transport, URL, or response-decoding error.
    pub async fn stream_with_cancellation(
        &self,
        request: &StreamRequest,
        cancellation: &CancellationToken,
    ) -> Result<ResponseStream, ApiError> {
        let link: StreamResponse = self
            .send_json(
                Method::POST,
                "/v1/data/stream",
                request,
                true,
                Some(cancellation),
            )
            .await?;
        let url = Url::parse(&link.link)
            .map_err(|error| ApiError::InvalidUrl(format!("invalid stream URL: {error}")))?;
        let response = send_with_cancellation(self.http.get(url), cancellation).await?;
        ensure_success_response(response)
            .await
            .map(|response| ResponseStream {
                response: Some(response),
                cancellation: cancellation.clone(),
            })
    }

    /// Upload a reader through the API's signed upload URL.
    ///
    /// # Errors
    ///
    /// Returns the mapped API, transport, URL, or upload-response error.
    pub async fn upload<R>(&self, reader: R, request: &UploadRequest) -> Result<(), ApiError>
    where
        R: AsyncRead + Send + 'static,
    {
        self.upload_with_cancellation(reader, request, &CancellationToken::new())
            .await
    }

    /// Upload a reader and cancel the active HTTP future when requested.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::Cancelled`] when cancellation wins, or the mapped
    /// API, transport, URL, or upload-response error.
    pub async fn upload_with_cancellation<R>(
        &self,
        reader: R,
        request: &UploadRequest,
        cancellation: &CancellationToken,
    ) -> Result<(), ApiError>
    where
        R: AsyncRead + Send + 'static,
    {
        let link: UploadResponse = self
            .send_json(
                Method::POST,
                "/v1/data/upload",
                request,
                true,
                Some(cancellation),
            )
            .await?;
        let url = Url::parse(&link.link)
            .map_err(|error| ApiError::InvalidUrl(format!("invalid upload URL: {error}")))?;
        let body = reqwest::Body::wrap_stream(ReaderStream::new(reader));
        let response = send_with_cancellation(
            self.http
                .put(url)
                .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
                .body(body),
            cancellation,
        )
        .await?;
        ensure_success(response).await
    }

    /// Download an attachment through the authenticated API.
    ///
    /// # Errors
    ///
    /// Returns the mapped API, transport, or URL error.
    pub async fn attachment(&self, id: &str) -> Result<ResponseStream, ApiError> {
        self.attachment_with_cancellation(id, &CancellationToken::new())
            .await
    }

    /// Download an attachment with cancellation support. Status is validated
    /// before its body is exposed to callers.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::Cancelled`] when cancellation wins, or the mapped
    /// API, transport, or URL error.
    pub async fn attachment_with_cancellation(
        &self,
        id: &str,
        cancellation: &CancellationToken,
    ) -> Result<ResponseStream, ApiError> {
        let endpoint = format!("/v1/recording-attachments/{id}/download");
        let response =
            send_with_cancellation(self.request(Method::GET, &endpoint)?, cancellation).await?;
        ensure_success_response(response)
            .await
            .map(|response| ResponseStream {
                response: Some(response),
                cancellation: cancellation.clone(),
            })
    }

    async fn send_json<B, T>(
        &self,
        method: Method,
        endpoint: &str,
        body: &B,
        authenticated: bool,
        cancellation: Option<&CancellationToken>,
    ) -> Result<T, ApiError>
    where
        B: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let request = self
            .request_with_auth(method, endpoint, authenticated)?
            .json(body);
        let response = send_with_optional_cancellation(request, cancellation).await?;
        let response = ensure_success_response(response).await?;
        response.json::<T>().await.map_err(ApiError::Decode)
    }

    async fn send_json_with_query<Q, B, T>(
        &self,
        method: Method,
        endpoint: &str,
        query: &Q,
        body: Option<&B>,
        cancellation: Option<&CancellationToken>,
    ) -> Result<T, ApiError>
    where
        Q: Serialize + ?Sized,
        B: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let mut request = self.request_with_auth(method, endpoint, true)?.query(query);
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = send_with_optional_cancellation(request, cancellation).await?;
        let response = ensure_success_response(response).await?;
        response.json::<T>().await.map_err(ApiError::Decode)
    }

    fn request(&self, method: Method, endpoint: &str) -> Result<RequestBuilder, ApiError> {
        self.request_with_auth(method, endpoint, true)
    }

    fn request_with_auth(
        &self,
        method: Method,
        endpoint: &str,
        authenticated: bool,
    ) -> Result<RequestBuilder, ApiError> {
        let url = self
            .base_url
            .join(endpoint.trim_start_matches('/'))
            .map_err(|error| ApiError::InvalidUrl(format!("invalid API endpoint: {error}")))?;
        let request = self
            .http
            .request(method, url)
            .header("User-Agent", &self.user_agent);
        let token = self
            .token
            .read()
            .map(|token| token.clone())
            .unwrap_or_default();
        if !authenticated || token.is_empty() {
            Ok(request)
        } else {
            Ok(request.bearer_auth(token))
        }
    }
}

/// A response body that can be consumed incrementally and cancelled safely.
pub struct ResponseStream {
    response: Option<Response>,
    cancellation: CancellationToken,
}

impl fmt::Debug for ResponseStream {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResponseStream")
            .finish_non_exhaustive()
    }
}

impl ResponseStream {
    /// Read the next body chunk, or `None` at EOF.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::Cancelled`] when the token is cancelled, or a
    /// transport error while reading the body.
    pub async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, ApiError> {
        if self.cancellation.is_cancelled() {
            self.response.take();
            return Err(ApiError::Cancelled);
        }
        let Some(response) = self.response.as_mut() else {
            return Ok(None);
        };
        let result = tokio::select! {
            () = self.cancellation.cancelled() => Err(ApiError::Cancelled),
            chunk = response.chunk() => chunk
                .map(|chunk| chunk.map(|chunk| chunk.to_vec()))
                .map_err(ApiError::Transport),
        };
        match result {
            Ok(Some(chunk)) => Ok(Some(chunk)),
            Ok(None) => {
                self.response.take();
                Ok(None)
            }
            Err(error) => {
                self.response.take();
                Err(error)
            }
        }
    }

    /// Copy the body to an async destination, stopping promptly on
    /// cancellation. The response is consumed or dropped on every return.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::Cancelled`], a transport error, or a destination
    /// write error.
    pub async fn copy_to<W>(&mut self, writer: &mut W) -> Result<u64, ApiError>
    where
        W: AsyncWrite + Unpin,
    {
        let mut written = 0_u64;
        while let Some(chunk) = self.next_chunk().await? {
            tokio::select! {
                () = self.cancellation.cancelled() => {
                    self.response.take();
                    return Err(ApiError::Cancelled);
                }
                result = writer.write_all(&chunk) => {
                    if let Err(error) = result {
                        self.response.take();
                        return Err(ApiError::Write(error));
                    }
                }
            }
            written += u64::try_from(chunk.len()).unwrap_or(u64::MAX);
        }
        Ok(written)
    }

    /// Read the complete body into memory.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::Cancelled`] when the token is cancelled or a
    /// transport error while reading the body.
    pub async fn bytes(mut self) -> Result<Vec<u8>, ApiError> {
        let mut output = Vec::new();
        while let Some(chunk) = self.next_chunk().await? {
            output.extend_from_slice(&chunk);
        }
        Ok(output)
    }
}

async fn send_with_optional_cancellation(
    request: RequestBuilder,
    cancellation: Option<&CancellationToken>,
) -> Result<Response, ApiError> {
    match cancellation {
        Some(cancellation) => send_with_cancellation(request, cancellation).await,
        None => request.send().await.map_err(ApiError::Transport),
    }
}

async fn send_with_cancellation(
    request: RequestBuilder,
    cancellation: &CancellationToken,
) -> Result<Response, ApiError> {
    if cancellation.is_cancelled() {
        return Err(ApiError::Cancelled);
    }
    tokio::select! {
        () = cancellation.cancelled() => Err(ApiError::Cancelled),
        response = request.send() => response.map_err(ApiError::Transport),
    }
}

async fn ensure_success(response: Response) -> Result<(), ApiError> {
    ensure_success_response(response).await.map(|_| ())
}

async fn ensure_success_response(response: Response) -> Result<Response, ApiError> {
    if response.status().is_success() {
        Ok(response)
    } else {
        Err(error_from_response(response).await)
    }
}

async fn error_from_response(response: Response) -> ApiError {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ApiError::Forbidden,
        StatusCode::NOT_FOUND => ApiError::NotFound,
        _ => ApiError::Response {
            status: status.as_u16(),
            message: response_message(&body),
        },
    }
}

fn response_message(body: &str) -> String {
    #[derive(Deserialize)]
    struct ErrorResponse {
        error: Option<String>,
        message: Option<String>,
    }
    match serde_json::from_str::<ErrorResponse>(body) {
        Ok(response) => response.error.or(response.message).unwrap_or_default(),
        Err(_) => body.to_owned(),
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_false(value: &bool) -> bool {
    !*value
}

#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_zero(value: &f64) -> bool {
    *value == 0.0
}

fn is_mcap_format(format: &str) -> bool {
    matches!(format, "mcap" | "mcap0")
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use tokio::io::AsyncWriteExt;

    use super::{response_message, ApiError, FoxgloveClient, StreamRequest};

    async fn request_head(stream: &mut tokio::net::TcpStream) -> String {
        use tokio::io::AsyncReadExt;

        let mut request = Vec::new();
        let mut byte = [0_u8; 1];
        while !request.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).await.unwrap();
            request.push(byte[0]);
        }
        String::from_utf8(request).unwrap()
    }

    async fn response(stream: &mut tokio::net::TcpStream, body: &str) {
        use tokio::io::AsyncWriteExt;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).await.unwrap();
    }

    async fn test_listener() -> (tokio::net::TcpListener, SocketAddr) {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let address = listener.local_addr().unwrap();
        (listener, address)
    }

    #[test]
    fn stream_request_uses_wire_names_and_omits_empty_options() {
        let request = StreamRequest {
            recording_id: "rec_1".into(),
            output_format: "mcap".into(),
            topics: vec![],
            ..StreamRequest::default()
        };
        let value = serde_json::to_value(request).unwrap();
        assert_eq!(value["recordingId"], "rec_1");
        assert_eq!(value["outputFormat"], "mcap");
        assert_eq!(value["topics"], serde_json::json!([]));
        assert!(value.get("projectId").is_none());
    }

    #[test]
    fn stream_request_serializes_rfc3339_timestamps() {
        let request = StreamRequest {
            recording_id: "rec_1".into(),
            output_format: "mcap".into(),
            start: Some(time::OffsetDateTime::from_unix_timestamp(0).unwrap()),
            topics: vec![],
            ..StreamRequest::default()
        };
        let value = serde_json::to_value(request).unwrap();
        assert_eq!(value["start"], "1970-01-01T00:00:00Z");
    }

    #[test]
    fn stream_request_validation_matches_api_contract() {
        let request = StreamRequest {
            output_format: "mcap0".into(),
            topics: vec![],
            ..StreamRequest::default()
        };
        assert!(request
            .validate()
            .unwrap_err()
            .contains("either recording-id/key"));
    }

    #[test]
    fn error_payload_prefers_error_then_message_then_raw_body() {
        assert_eq!(response_message(r#"{"error":"bad"}"#), "bad");
        assert_eq!(response_message(r#"{"message":"bad"}"#), "bad");
        assert_eq!(response_message("bad"), "bad");
    }

    #[test]
    fn status_helpers_are_available_without_string_matching() {
        assert!(ApiError::Forbidden.is_forbidden());
        assert!(ApiError::NotFound.is_not_found());
        assert!(ApiError::Cancelled.is_cancelled());
    }

    #[tokio::test]
    async fn cancellation_short_circuits_before_network_io() {
        let cancellation = tokio_util::sync::CancellationToken::new();
        cancellation.cancel();
        let client = FoxgloveClient::new(
            "http://127.0.0.1:1",
            "client",
            "fixture-token",
            "foxglove-cli/test",
        )
        .unwrap();
        let error = client
            .get_with_cancellation::<(), serde_json::Value>("/v1/me", &(), &cancellation)
            .await
            .unwrap_err();
        assert!(error.is_cancelled());
    }

    #[ignore = "requires loopback socket access; run explicitly in the wire-test environment"]
    #[tokio::test]
    async fn signin_is_unauthenticated_then_reuses_bearer_token() {
        let (listener, address) = test_listener().await;
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let request = request_head(&mut stream).await.to_ascii_lowercase();
            assert!(request.contains("post /v1/signin "));
            assert!(!request.contains("authorization:"));
            assert!(request.contains("user-agent: foxglove-cli/test"));
            response(&mut stream, r#"{"bearerToken":"fixture-token"}"#).await;

            let (mut stream, _) = listener.accept().await.unwrap();
            let request = request_head(&mut stream).await.to_ascii_lowercase();
            assert!(request.contains("get /v1/me "));
            assert!(request.contains("authorization: bearer fixture-token"));
            response(&mut stream, r#"{"id":"user"}"#).await;
        });

        let client = FoxgloveClient::new(
            &format!("http://{address}"),
            "client",
            "",
            "foxglove-cli/test",
        )
        .unwrap();
        assert_eq!(client.sign_in("id-token").await.unwrap(), "fixture-token");
        let me: serde_json::Value = client.get("/v1/me", &()).await.unwrap();
        assert_eq!(me["id"], "user");
        server.await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires loopback socket access; run explicitly in the wire-test environment"]
    async fn stream_follows_link_and_exposes_incremental_body() {
        let (listener, address) = test_listener().await;
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let request = request_head(&mut stream).await.to_ascii_lowercase();
            assert!(request.contains("post /v1/data/stream "));
            assert!(request.contains("authorization: bearer fixture-token"));
            response(
                &mut stream,
                &format!(r#"{{"link":"http://{address}/storage/fixture"}}"#),
            )
            .await;

            let (mut stream, _) = listener.accept().await.unwrap();
            let request = request_head(&mut stream).await.to_ascii_lowercase();
            assert!(request.contains("get /storage/fixture "));
            assert!(!request.contains("authorization:"));
            let body = b"fixture stream";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            stream.write_all(body).await.unwrap();
        });

        let client = FoxgloveClient::new(
            &format!("http://{address}"),
            "client",
            "fixture-token",
            "foxglove-cli/test",
        )
        .unwrap();
        let request = StreamRequest {
            recording_id: "recording".into(),
            output_format: "mcap".into(),
            topics: vec![],
            ..StreamRequest::default()
        };
        let stream = client.stream(&request).await.unwrap();
        assert_eq!(stream.bytes().await.unwrap(), b"fixture stream");
        server.await.unwrap();
    }
}
