//! Authentication commands.

use serde::Deserialize;
use serde_yaml_ng::Value;
use std::io::Write;
use std::process::Child;
#[cfg(not(feature = "compat-test"))]
use std::process::Command;
use std::time::Duration;

use crate::api::{self, FoxgloveClient};
use crate::cli::LoginArgs;
use crate::config::Config;
use crate::output;
use crate::runtime::{self, Runtime};
use crate::Outcome;

#[derive(Deserialize)]
struct MeResponse {
    email: String,
    #[serde(rename = "emailVerified")]
    email_verified: bool,
    #[serde(rename = "orgId")]
    org_id: String,
    #[serde(rename = "orgSlug")]
    org_slug: String,
    admin: bool,
}

pub(crate) async fn info(runtime: &Runtime) -> Outcome {
    let token = runtime
        .config
        .get_string("bearer_token")
        .unwrap_or_default();
    if token.is_empty() {
        return Outcome::failure(
            "Not signed in. Run `foxglove auth login` or `foxglove auth configure-api-key` to continue.\n",
        );
    }
    let auth_type = runtime
        .config
        .get_string("auth_type")
        .and_then(|value| value.parse::<i32>().ok());
    if auth_type == Some(2) || (auth_type != Some(1) && token.starts_with("fox_sk_")) {
        return Outcome::success("Authenticated with API key\n");
    }
    match runtime
        .client
        .get::<_, MeResponse>("/v1/me", &Vec::<(String, String)>::new())
        .await
    {
        Ok(me) => {
            let mut stdout = b"Authenticated with session token\n".to_vec();
            let headers = ["Email", "Email verified", "Org ID", "Org Slug", "Admin"];
            let rows = [vec![
                me.email,
                me.email_verified.to_string(),
                me.org_id,
                me.org_slug,
                me.admin.to_string(),
            ]];
            match output::render_table(&mut stdout, &headers, &rows) {
                Ok(()) => Outcome::success(stdout),
                Err(error) => Outcome::failure(format!("Info command failed: {error}\n")),
            }
        }
        Err(error) => Outcome::failure(format!("Info command failed: {error}\n")),
    }
}

/// Run the browser-based device-code login flow and persist its session token.
pub(crate) async fn login(
    args: &LoginArgs,
    config_path: Option<&std::path::Path>,
    client_id: Option<&str>,
    prompt_writer: &mut dyn Write,
) -> Outcome {
    let base_url = args
        .base_url
        .clone()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| runtime::DEFAULT_BASE_URL.to_owned());
    let client = match FoxgloveClient::new(
        &base_url,
        client_id.unwrap_or(runtime::DEFAULT_CLIENT_ID),
        "",
        runtime::user_agent(),
    ) {
        Ok(client) => client,
        Err(error) => return Outcome::failure(format!("Login failed: {error}\n")),
    };

    let (instructions, device_code, browser) = match start_login(&client).await {
        Ok(result) => result,
        Err(error) => return Outcome::failure(format!("Login failed: {error}\n")),
    };
    if let Err(error) = prompt_writer
        .write_all(&instructions)
        .and_then(|()| prompt_writer.flush())
    {
        stop_browser(browser);
        return Outcome::failure(format!("failed to write login instructions: {error}\n"));
    }
    let bearer_token = match complete_login(&client, &device_code, browser).await {
        Ok(token) => token,
        Err(error) => return Outcome::failure(format!("Login failed: {error}\n")),
    };
    let mut config = match Config::load_from_path(config_path) {
        Ok(config) => config,
        Err(error) => return Outcome::failure(format!("Login failed: {error}\n")),
    };
    config.set("auth_type", Value::Number(1.into()));
    config.set("base_url", Value::String(base_url));
    config.set("bearer_token", Value::String(bearer_token));
    match config.save() {
        Ok(()) => Outcome::default(),
        Err(error) => Outcome {
            stderr: format!(
                "Login failed: failed to configure auth: {}\n",
                error.trim_end()
            )
            .into_bytes(),
            exit_code: 1,
            ..Outcome::default()
        },
    }
}

async fn start_login(
    client: &FoxgloveClient,
) -> Result<(Vec<u8>, crate::api::DeviceCodeResponse, Option<Child>), String> {
    let device_code = client
        .device_code()
        .await
        .map_err(|error| format!("failed to fetch device code: {error}"))?;
    let browser = open_browser(&device_code.verification_uri_complete);
    let mut stdout = Vec::new();
    if browser.is_some() {
        stdout.extend_from_slice(
            b"If no window opens, copy/paste the following link into your browser:\n",
        );
    } else {
        stdout.extend_from_slice(b"copy/paste the following link into your browser:\n");
    }
    stdout.extend_from_slice(b"\n");
    stdout.extend_from_slice(device_code.verification_uri_complete.as_bytes());
    stdout.extend_from_slice(b"\n\nVerify this code and click 'Authorize' to complete login:  ");
    stdout.extend_from_slice(device_code.user_code.as_bytes());
    stdout.extend_from_slice(b"\n");

    Ok((stdout, device_code, browser))
}

async fn complete_login(
    client: &FoxgloveClient,
    device_code: &crate::api::DeviceCodeResponse,
    browser: Option<Child>,
) -> Result<String, String> {
    let cancellation = api::ctrl_c_cancellation_token();
    let result = async {
        loop {
            if cancellation.is_cancelled() {
                return Err("context canceled".to_owned());
            }
            match client
                .token_with_cancellation(&device_code.device_code, &cancellation)
                .await
            {
                Ok(token) => break Ok(token),
                Err(error) if error.is_forbidden() => {
                    tokio::select! {
                        () = cancellation.cancelled() => return Err("context canceled".to_owned()),
                        () = tokio::time::sleep(Duration::from_millis(500)) => {}
                    }
                }
                Err(error) => return Err(format!("failed to request token: {error}")),
            }
        }
    }
    .await;
    stop_browser(browser);
    let token = result?;
    let bearer_token = client
        .sign_in(&token)
        .await
        .map_err(|error| format!("failed to sign in: {error}"))?;
    Ok(bearer_token)
}

#[cfg(feature = "compat-test")]
fn open_browser(_url: &str) -> Option<Child> {
    None
}

#[cfg(not(feature = "compat-test"))]
fn open_browser(url: &str) -> Option<Child> {
    #[cfg(target_os = "linux")]
    let mut command = Command::new("xdg-open");
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("rundll32");
        command.arg("url.dll,FileProtocolHandler");
        command
    };
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    return None;
    command.arg(url).spawn().ok()
}

fn stop_browser(browser: Option<Child>) {
    if let Some(mut browser) = browser {
        let _ = browser.kill();
    }
}
