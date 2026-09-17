//! Asynchronous Foxglove Data Platform API client.
//!
//! The client deliberately keeps the HTTP and transfer concerns separate from
//! command execution. Commands can therefore stage output and decide their own
//! cleanup policy while this module guarantees that response bodies are either
//! consumed or dropped on every branch.
//!
//! Keep static endpoint paths as strings and percent-encode dynamic identifiers
//! with `encode_path_segment(value)` before interpolating them.
//! Pass query parameters separately.

use std::fmt;
use std::io;
use std::sync::{Arc, RwLock};

use percent_encoding::{utf8_percent_encode, AsciiSet, PercentEncode, NON_ALPHANUMERIC};
use reqwest::{Method, RequestBuilder, Response, StatusCode, Url};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio_util::io::ReaderStream;
use tokio_util::sync::CancellationToken;

// Encode every byte except RFC 3986 unreserved characters. In particular,
// percent signs are literal input, never already-escaped URL syntax.
const PATH_SEGMENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

/// Percent-encode a raw identifier for use as a single URL path segment.
pub(crate) fn encode_path_segment(value: &str) -> PercentEncode<'_> {
    utf8_percent_encode(value, PATH_SEGMENT)
}

const FORBIDDEN_MESSAGE: &str = "forbidden: have you signed in with `foxglove auth login`?";

/// Errors returned by the API client.
#[derive(Debug)]
pub enum ApiError {
    /// Context retained by an endpoint-specific operation around a typed error.
    Context {
        context: &'static str,
        source: Box<ApiError>,
    },
    /// The API rejected the credentials with HTTP 401.
    Unauthorized,
    /// The API rejected the credentials with HTTP 403.
    Forbidden,
    /// The API rejected the credentials and supplied additional detail.
    ForbiddenWithMessage(String),
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
    /// A streamed export could not be converted to the requested local form.
    Conversion(String),
    /// A streamed destination rejected a chunk.
    Write(io::Error),
    /// A signed storage upload did not return the required HTTP 200 response.
    UnexpectedUploadStatus(u16),
}

impl ApiError {
    fn contextualize(self, transport: &'static str, decode: &'static str) -> Self {
        match self {
            Self::Transport(_) => Self::Context {
                context: transport,
                source: Box::new(self),
            },
            Self::Decode(_) => Self::Context {
                context: decode,
                source: Box::new(self),
            },
            _ => self,
        }
    }
    /// Whether the server rejected the current authentication.
    #[must_use]
    pub const fn is_forbidden(&self) -> bool {
        matches!(
            self,
            Self::Unauthorized | Self::Forbidden | Self::ForbiddenWithMessage(_)
        )
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

    /// Whether the same request could succeed on a later attempt.
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Context { source, .. } => source.is_retryable(),
            Self::Transport(_) => true,
            Self::Response { status, .. } => *status == 429 || *status >= 500,
            _ => false,
        }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Context { context, source } => write!(formatter, "{context}: {source}"),
            Self::Unauthorized | Self::Forbidden => formatter.write_str(FORBIDDEN_MESSAGE),
            Self::ForbiddenWithMessage(message) => {
                write!(formatter, "{FORBIDDEN_MESSAGE}\n{message}")
            }
            Self::NotFound => formatter.write_str("not found"),
            Self::Response { message, .. } => formatter.write_str(message),
            Self::Transport(error) | Self::Decode(error) => error.fmt(formatter),
            Self::Serialization(error) => error.fmt(formatter),
            Self::Cancelled => formatter.write_str("operation cancelled"),
            Self::InvalidUrl(error) | Self::Conversion(error) => formatter.write_str(error),
            Self::Write(error) => error.fmt(formatter),
            Self::UnexpectedUploadStatus(status) => {
                write!(formatter, "unexpected {status} on upload request")
            }
        }
    }
}

impl std::error::Error for ApiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Context { source, .. } => Some(source),
            Self::Transport(error) | Self::Decode(error) => Some(error),
            Self::Serialization(error) => Some(error),
            Self::Write(error) => Some(error),
            Self::Unauthorized
            | Self::Forbidden
            | Self::ForbiddenWithMessage(_)
            | Self::NotFound
            | Self::Response { .. }
            | Self::Cancelled
            | Self::InvalidUrl(_)
            | Self::Conversion(_)
            | Self::UnexpectedUploadStatus(_) => None,
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
    pub episode_id: String,
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
        let episode = !self.episode_id.is_empty();
        if !(recording || session || device || import || episode) {
            return Err("either recording-id/key, session-id/session-key, import-id, episode-id, or device-id/device-name with start/end are required".to_owned());
        }
        if !self.session_key.is_empty() && self.project_id.is_empty() {
            return Err("project-id is required when using session-key".to_owned());
        }
        if device
            && !import
            && !recording
            && !session
            && !episode
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
    #[serde(default)]
    pub user_code: String,
    #[serde(default)]
    pub expires_in: u64,
    #[serde(default)]
    pub interval: u64,
    #[serde(default)]
    pub verification_uri: String,
    #[serde(default)]
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
        self.sign_in_with_cancellation(id_token, &CancellationToken::new())
            .await
    }

    /// Execute `sign_in` with cancellation covering headers and response bodies.
    ///
    /// # Errors
    ///
    /// Returns cancellation, transport, API, or decoding errors.
    pub async fn sign_in_with_cancellation(
        &self,
        id_token: &str,
        cancellation: &CancellationToken,
    ) -> Result<String, ApiError> {
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
                Some(cancellation),
            )
            .await
            .map_err(|error| {
                error.contextualize("sign in failure", "failed to decode sign in response")
            })?;
        self.set_token(response.token.clone());
        Ok(response.token)
    }

    /// Fetch a device code for the interactive login flow.
    ///
    /// # Errors
    ///
    /// Returns the mapped API, transport, or response-decoding error.
    pub async fn device_code(&self) -> Result<DeviceCodeResponse, ApiError> {
        self.device_code_with_cancellation(&CancellationToken::new())
            .await
    }

    /// Execute `device_code` with cancellation covering headers and response bodies.
    ///
    /// # Errors
    ///
    /// Returns cancellation, transport, API, or decoding errors.
    pub async fn device_code_with_cancellation(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<DeviceCodeResponse, ApiError> {
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
            Some(cancellation),
        )
        .await
        .map_err(|error| {
            error.contextualize("failed to fetch device code", "failed to decode response")
        })
    }

    /// Poll for the ID token associated with a device code.
    ///
    /// # Errors
    ///
    /// Returns the mapped API, transport, or response-decoding error.
    pub async fn token(&self, device_code: &str) -> Result<String, ApiError> {
        self.token_with_cancellation(device_code, &CancellationToken::new())
            .await
    }

    /// Poll for an ID token, aborting the active HTTP request on cancellation.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::Cancelled`] when cancellation wins, or the mapped
    /// API, transport, or response-decoding error.
    pub async fn token_with_cancellation(
        &self,
        device_code: &str,
        cancellation: &CancellationToken,
    ) -> Result<String, ApiError> {
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
        let request = self
            .request_with_auth(Method::POST, "/v1/auth/token", false)?
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(encode_json(&TokenRequest {
                client_id: &self.client_id,
                device_code,
            })?);
        let response = send_with_cancellation(request, cancellation)
            .await
            .map_err(|error| {
                error.contextualize("token request failure", "failed to parse response body")
            })?;
        with_optional_cancellation(Some(cancellation), async {
            match response.status() {
                StatusCode::OK => response
                    .json::<TokenResponse>()
                    .await
                    .map(|response| response.id_token)
                    .map_err(ApiError::Decode)
                    .map_err(|error| {
                        error
                            .contextualize("token request failure", "failed to parse response body")
                    }),
                StatusCode::UNAUTHORIZED => {
                    drop(response);
                    Err(ApiError::Unauthorized)
                }
                StatusCode::FORBIDDEN => {
                    drop(response);
                    Err(ApiError::Forbidden)
                }
                status => {
                    let _ = response.text().await;
                    Err(ApiError::Response {
                        status: status.as_u16(),
                        message: format!("unexpected status {}", status.as_u16()),
                    })
                }
            }
        })
        .await
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

    /// Execute an authenticated DELETE with query parameters and consume its
    /// response body.
    ///
    /// # Errors
    ///
    /// Returns the mapped API, serialization, or transport error.
    pub async fn delete_with_query<Q>(&self, endpoint: &str, query: &Q) -> Result<(), ApiError>
    where
        Q: Serialize + ?Sized,
    {
        let request = self.request(Method::DELETE, endpoint)?.query(query);
        let response = send_with_cancellation(request, &CancellationToken::new()).await?;
        ensure_ok(response).await
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
        with_optional_cancellation(Some(cancellation), ensure_ok(response)).await
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
        // Signed storage URLs must not inherit the API bearer token, but they
        // use the same CLI identity as ordinary requests.
        let response = send_with_cancellation(
            self.http
                .get(url)
                .header(reqwest::header::USER_AGENT, &self.user_agent),
            cancellation,
        )
        .await?;
        with_optional_cancellation(Some(cancellation), ensure_success_response(response))
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

    /// Upload an extension package directly to the authenticated API.
    ///
    /// # Errors
    ///
    /// Returns the mapped API or transport error.
    pub async fn upload_extension<R>(&self, reader: R) -> Result<(), ApiError>
    where
        R: AsyncRead + Send + 'static,
    {
        self.upload_extension_with_cancellation(reader, &CancellationToken::new())
            .await
    }

    /// Upload an extension package, cancelling the active HTTP request when
    /// the supplied token is cancelled.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::Cancelled`] when cancellation wins, or the mapped
    /// API or transport error.
    pub async fn upload_extension_with_cancellation<R>(
        &self,
        reader: R,
        cancellation: &CancellationToken,
    ) -> Result<(), ApiError>
    where
        R: AsyncRead + Send + 'static,
    {
        let request = self
            .request(Method::POST, "/v1/extension-upload")?
            .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
            .body(reqwest::Body::wrap_stream(ReaderStream::new(reader)));
        let response = send_with_cancellation(request, cancellation).await?;
        with_optional_cancellation(Some(cancellation), ensure_ok(response)).await
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
                // Signed storage links are followed outside the authenticated
                // API client, without its bearer token but with its identity.
                .header(reqwest::header::USER_AGENT, &self.user_agent)
                .body(body),
            cancellation,
        )
        .await?;
        with_optional_cancellation(Some(cancellation), ensure_upload_success(response)).await
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
        let endpoint = format!(
            "/v1/recording-attachments/{}/download",
            encode_path_segment(id)
        );
        let response =
            send_with_cancellation(self.request(Method::GET, &endpoint)?, cancellation).await?;
        with_optional_cancellation(Some(cancellation), ensure_success_response(response))
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
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(encode_json(body)?);
        let response = send_with_optional_cancellation(request, cancellation).await?;
        with_optional_cancellation(cancellation, async {
            let response = ensure_ok_response(response).await?;
            response.json::<T>().await.map_err(ApiError::Decode)
        })
        .await
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
            request = request
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(encode_json(body)?);
        }
        let response = send_with_optional_cancellation(request, cancellation).await?;
        with_optional_cancellation(cancellation, async {
            let response = ensure_ok_response(response).await?;
            response.json::<T>().await.map_err(ApiError::Decode)
        })
        .await
    }

    /// Reject ambiguous segments before URL parsing can normalize them away.
    fn endpoint_url(&self, endpoint: &str) -> Result<Url, ApiError> {
        let path = endpoint.trim_start_matches('/');
        if path
            .split('/')
            .any(|segment| matches!(segment, "" | "." | ".."))
        {
            return Err(ApiError::InvalidUrl(
                "API path segments must not be empty, '.' or '..'".into(),
            ));
        }
        self.base_url
            .join(path)
            .map_err(|error| ApiError::InvalidUrl(format!("invalid API endpoint: {error}")))
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
        let url = self.endpoint_url(endpoint)?;
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

fn encode_json<T>(body: &T) -> Result<Vec<u8>, ApiError>
where
    T: Serialize + ?Sized,
{
    let mut encoded = serde_json::to_vec(body).map_err(ApiError::Serialization)?;
    encoded.push(b'\n');
    Ok(encoded)
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
    with_optional_cancellation(Some(cancellation), async {
        request.send().await.map_err(ApiError::Transport)
    })
    .await
}

/// Keep cancellation active while consuming success and error response bodies,
/// not just while waiting for response headers.
async fn with_optional_cancellation<T>(
    cancellation: Option<&CancellationToken>,
    operation: impl std::future::Future<Output = Result<T, ApiError>>,
) -> Result<T, ApiError> {
    let Some(cancellation) = cancellation else {
        return operation.await;
    };
    tokio::select! {
        biased;
        () = cancellation.cancelled() => Err(ApiError::Cancelled),
        result = operation => result,
    }
}

async fn ensure_ok(response: Response) -> Result<(), ApiError> {
    ensure_ok_response(response).await.map(|_| ())
}

async fn ensure_upload_success(response: Response) -> Result<(), ApiError> {
    if response.status() == StatusCode::OK {
        drop(response);
        Ok(())
    } else {
        let status = response.status().as_u16();
        let _ = response.text().await;
        Err(ApiError::UnexpectedUploadStatus(status))
    }
}

async fn ensure_success_response(response: Response) -> Result<Response, ApiError> {
    if response.status().is_success() {
        Ok(response)
    } else {
        Err(error_from_response(response).await)
    }
}

async fn ensure_ok_response(response: Response) -> Result<Response, ApiError> {
    if response.status() == StatusCode::OK {
        Ok(response)
    } else {
        Err(error_from_response(response).await)
    }
}

async fn error_from_response(response: Response) -> ApiError {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    api_error_from_response(status, &body)
}

fn api_error_from_response(status: StatusCode, body: &str) -> ApiError {
    match status {
        StatusCode::UNAUTHORIZED => ApiError::Unauthorized,
        StatusCode::FORBIDDEN => {
            let message = response_message(body);
            if message.is_empty() {
                ApiError::Forbidden
            } else {
                ApiError::ForbiddenWithMessage(message)
            }
        }
        StatusCode::NOT_FOUND => ApiError::NotFound,
        _ => ApiError::Response {
            status: status.as_u16(),
            message: response_message(body),
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
        Ok(response) => response
            .error
            .filter(|value| !value.is_empty())
            .or(response.message.filter(|value| !value.is_empty()))
            .unwrap_or_default(),
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

    use reqwest::StatusCode;
    use tokio::io::AsyncWriteExt;

    use super::{
        api_error_from_response, encode_path_segment, response_message, ApiError, FoxgloveClient,
        StreamRequest, FORBIDDEN_MESSAGE,
    };

    #[test]
    fn api_paths_preserve_raw_identifiers_and_base_path() {
        let client = FoxgloveClient::new(
            "https://example.test/proxy%20prefix?unused=yes#unused",
            "client",
            "token",
            "test",
        )
        .unwrap();
        for key in [
            "drive#1",
            "a/b\\c?x=y&z",
            "%2e%2f",
            " space ",
            "雪☃",
            "a\r\n\tb",
        ] {
            let request = client
                .request(
                    reqwest::Method::DELETE,
                    &format!("/v1/sessions/{}", encode_path_segment(key)),
                )
                .unwrap()
                .query(&[("projectId", "a&b=#/?% 雪")])
                .build()
                .unwrap();
            let url = request.url();
            assert_eq!(url.origin(), client.base_url.origin());
            assert_eq!(url.fragment(), None);
            let path = url
                .path()
                .strip_prefix("/proxy%20prefix/v1/sessions/")
                .unwrap();
            assert!(!path.contains('/'), "{key:?}");
            assert_eq!(
                percent_encoding::percent_decode_str(path)
                    .decode_utf8()
                    .unwrap(),
                key
            );
            assert_eq!(
                url.query_pairs().collect::<Vec<_>>(),
                vec![("projectId".into(), "a&b=#/?% 雪".into())]
            );
        }
        assert_eq!(
            client
                .endpoint_url(&format!("/v1/sessions/{}", encode_path_segment("drive#1")))
                .unwrap()
                .as_str(),
            "https://example.test/proxy%20prefix/v1/sessions/drive%231"
        );
        assert_eq!(
            client
                .endpoint_url(&format!("/v1/sessions/{}", encode_path_segment("%2e")))
                .unwrap()
                .path(),
            "/proxy%20prefix/v1/sessions/%252e"
        );
    }

    #[test]
    fn api_paths_encode_all_ascii_bytes_without_changing_identifiers() {
        let client =
            FoxgloveClient::new("https://example.test", "client", "token", "test").unwrap();
        for byte in 0..=127_u8 {
            let key = format!("a{}b", char::from(byte));
            let url = client
                .endpoint_url(&format!(
                    "/v1/sessions/{}/recordings",
                    encode_path_segment(&key)
                ))
                .unwrap();
            let segments: Vec<_> = url.path_segments().unwrap().collect();
            assert_eq!(segments.len(), 4, "{key:?}");
            assert_eq!(
                percent_encoding::percent_decode_str(segments[2])
                    .decode_utf8()
                    .unwrap(),
                key
            );
            assert_eq!(segments[3], "recordings");
            assert!(url.query().is_none());
            assert!(url.fragment().is_none());
        }
    }

    #[test]
    fn api_paths_reject_segments_that_would_target_a_different_resource() {
        let client =
            FoxgloveClient::new("https://example.test", "client", "token", "test").unwrap();
        for key in ["", ".", ".."] {
            assert!(matches!(
                client.request(
                    reqwest::Method::DELETE,
                    &format!("/v1/sessions/{}", encode_path_segment(key))
                ),
                Err(ApiError::InvalidUrl(_))
            ));
        }
    }

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
        assert_eq!(
            response_message(r#"{"error":"","message":"fallback"}"#),
            "fallback"
        );
        assert_eq!(response_message("bad"), "bad");
    }

    #[test]
    fn unauthorized_errors_always_direct_users_to_sign_in() {
        let mutation = api_error_from_response(
            StatusCode::UNAUTHORIZED,
            r#"{"error":"mutation requires authentication"}"#,
        );
        assert!(matches!(mutation, ApiError::Unauthorized));
        assert_eq!(mutation.to_string(), FORBIDDEN_MESSAGE);

        let authentication = api_error_from_response(
            StatusCode::UNAUTHORIZED,
            r#"{"error":"read requires authentication"}"#,
        );
        assert!(matches!(authentication, ApiError::Unauthorized));
    }

    #[test]
    fn status_helpers_are_available_without_string_matching() {
        assert!(ApiError::Forbidden.is_forbidden());
        assert!(ApiError::Unauthorized.is_forbidden());
        let forbidden = ApiError::ForbiddenWithMessage("requires capability".to_owned());
        assert!(forbidden.is_forbidden());
        assert_eq!(
            forbidden.to_string(),
            "forbidden: have you signed in with `foxglove auth login`?\nrequires capability"
        );
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

    async fn stall_response_body(
        listener: tokio::net::TcpListener,
        cancellation: tokio_util::sync::CancellationToken,
        status: u16,
    ) {
        let (mut stream, _) = listener.accept().await.unwrap();
        request_head(&mut stream).await;
        stream.write_all(format!("HTTP/1.1 {status} Fixture\r\nContent-Length: 1000\r\nConnection: close\r\n\r\n{{").as_bytes()).await.unwrap();
        // Only cancellation can finish the body read before the test timeout.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        cancellation.cancel();
        std::future::pending::<()>().await;
        drop(stream);
    }

    #[ignore = "requires loopback socket access; run explicitly in the wire-test environment"]
    #[tokio::test]
    async fn cancellation_interrupts_success_and_error_response_bodies() {
        use std::time::Duration;
        use tokio_util::sync::CancellationToken;
        for operation in [
            "get",
            "post",
            "patch",
            "delete",
            "attachment",
            "extension",
            "signin",
            "device-code",
            "token",
        ] {
            for status in [200, 500] {
                // Successful DELETE/upload responses are deliberately dropped.
                if status == 200 && matches!(operation, "delete" | "extension") {
                    continue;
                }
                let (listener, address) = test_listener().await;
                let cancellation = CancellationToken::new();
                let cancel = cancellation.clone();
                let server = tokio::spawn(stall_response_body(listener, cancel, status));
                let client = FoxgloveClient::new(
                    &format!("http://{address}"),
                    "fixture",
                    "",
                    "foxglove-cli/test",
                )
                .unwrap();
                let result = tokio::time::timeout(Duration::from_secs(3), async {
                    match operation {
                        "get" => client
                            .get_with_cancellation::<_, serde_json::Value>(
                                "/fixture",
                                &(),
                                &cancellation,
                            )
                            .await
                            .map(|_| ()),
                        "post" => client
                            .post_with_cancellation::<_, serde_json::Value>(
                                "/fixture",
                                &(),
                                &cancellation,
                            )
                            .await
                            .map(|_| ()),
                        "patch" => client
                            .patch_with_cancellation::<_, _, serde_json::Value>(
                                "/fixture",
                                &(),
                                &(),
                                &cancellation,
                            )
                            .await
                            .map(|_| ()),
                        "delete" => {
                            client
                                .delete_with_cancellation("/fixture", &cancellation)
                                .await
                        }
                        "attachment" => match client
                            .attachment_with_cancellation("fixture", &cancellation)
                            .await
                        {
                            Ok(stream) => stream.bytes().await.map(|_| ()),
                            Err(error) => Err(error),
                        },
                        "extension" => {
                            client
                                .upload_extension_with_cancellation(
                                    tokio::io::empty(),
                                    &cancellation,
                                )
                                .await
                        }
                        "signin" => client
                            .sign_in_with_cancellation("fixture", &cancellation)
                            .await
                            .map(|_| ()),
                        "device-code" => client
                            .device_code_with_cancellation(&cancellation)
                            .await
                            .map(|_| ()),
                        "token" => client
                            .token_with_cancellation("fixture", &cancellation)
                            .await
                            .map(|_| ()),
                        _ => unreachable!(),
                    }
                })
                .await;
                server.abort();
                let error = result
                    .unwrap_or_else(|_| panic!("{operation} {status} ignored cancellation"))
                    .unwrap_err();
                assert!(error.is_cancelled(), "{operation} {status}: {error}");
            }
        }
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
            assert!(request.contains("user-agent: foxglove-cli/test"));
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
