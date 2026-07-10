use crate::usage::{RateLimitBucket, SharedUsageState, UsageSnapshot};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::HashMap, env, path::PathBuf, process::Stdio, time::Duration};
use tauri::{AppHandle, Emitter};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{ChildStdout, Command},
    time::{interval, sleep, timeout},
};

const EVENT_NAME: &str = "usage-updated";
const RECONNECT_DELAY: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(12);
const REFRESH_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RateLimitsReadResult {
    rate_limits: Option<RateLimitBucket>,
    #[serde(default)]
    rate_limits_by_limit_id: HashMap<String, RateLimitBucket>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RateLimitsUpdatedParams {
    rate_limits: RateLimitBucket,
}

pub fn spawn(app: AppHandle, state: SharedUsageState) {
    tauri::async_runtime::spawn(async move {
        loop {
            update_state(&app, &state, |snapshot| snapshot.set_connecting()).await;

            if let Err(error) = run_session(&app, &state).await {
                update_state(&app, &state, |snapshot| snapshot.set_error(error)).await;
            }

            sleep(RECONNECT_DELAY).await;
        }
    });
}

async fn run_session(app: &AppHandle, state: &SharedUsageState) -> Result<(), String> {
    let codex_binary = find_codex_binary();
    let mut command = Command::new(&codex_binary);
    command
        .args(["app-server", "--listen", "stdio://"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);

    let mut child = command.spawn().map_err(|error| {
        format!(
            "Could not start Codex App Server at {}: {error}",
            codex_binary.display()
        )
    })?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "Codex App Server stdin was unavailable".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Codex App Server stdout was unavailable".to_string())?;
    let mut lines = BufReader::new(stdout).lines();

    write_message(
        &mut stdin,
        &json!({
            "method": "initialize",
            "id": 0,
            "params": {
                "clientInfo": {
                    "name": "codex_usage_dock",
                    "title": "Codex Usage Dock",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "capabilities": {
                    "optOutNotificationMethods": [
                        "thread/started",
                        "item/agentMessage/delta",
                        "remoteControl/status/changed"
                    ]
                }
            }
        }),
    )
    .await?;

    timeout(REQUEST_TIMEOUT, wait_for_response(&mut lines, 0))
        .await
        .map_err(|_| "Codex App Server initialization timed out".to_string())??;

    write_message(
        &mut stdin,
        &json!({ "method": "initialized", "params": {} }),
    )
    .await?;

    let mut request_id = 1_u64;
    request_rate_limits(&mut stdin, request_id).await?;

    let mut refresh = interval(REFRESH_INTERVAL);
    refresh.tick().await;

    loop {
        tokio::select! {
            line = lines.next_line() => {
                let line = line
                    .map_err(|error| format!("Could not read Codex App Server output: {error}"))?
                    .ok_or_else(|| "Codex App Server stopped".to_string())?;
                handle_message(app, state, &line).await?;
            }
            _ = refresh.tick() => {
                request_id += 1;
                request_rate_limits(&mut stdin, request_id).await?;
            }
        }
    }
}

async fn wait_for_response(
    lines: &mut Lines<BufReader<ChildStdout>>,
    expected_id: u64,
) -> Result<Value, String> {
    loop {
        let line = lines
            .next_line()
            .await
            .map_err(|error| format!("Could not read Codex App Server output: {error}"))?
            .ok_or_else(|| "Codex App Server stopped during initialization".to_string())?;
        let value: Value = serde_json::from_str(&line)
            .map_err(|error| format!("Codex App Server returned invalid JSON: {error}"))?;

        if value.get("id").and_then(Value::as_u64) == Some(expected_id) {
            if let Some(error) = value.get("error") {
                return Err(format!("Codex App Server rejected initialization: {error}"));
            }

            return Ok(value.get("result").cloned().unwrap_or(Value::Null));
        }
    }
}

async fn request_rate_limits(
    stdin: &mut tokio::process::ChildStdin,
    id: u64,
) -> Result<(), String> {
    write_message(
        stdin,
        &json!({ "method": "account/rateLimits/read", "id": id }),
    )
    .await
}

async fn handle_message(
    app: &AppHandle,
    state: &SharedUsageState,
    line: &str,
) -> Result<(), String> {
    let value: Value = serde_json::from_str(line)
        .map_err(|error| format!("Codex App Server returned invalid JSON: {error}"))?;

    if value.get("id").and_then(Value::as_u64).is_some() {
        if let Some(error) = value.get("error") {
            return Err(format!("Could not read Codex usage: {error}"));
        }

        if let Some(result) = value.get("result") {
            let response: RateLimitsReadResult = serde_json::from_value(result.clone())
                .map_err(|error| format!("Could not parse Codex usage: {error}"))?;
            if let Some(bucket) = select_main_bucket(response) {
                apply_bucket(app, state, bucket).await;
            }
        }
    } else if value.get("method").and_then(Value::as_str) == Some("account/rateLimits/updated") {
        let params: RateLimitsUpdatedParams =
            serde_json::from_value(value.get("params").cloned().unwrap_or(Value::Null))
                .map_err(|error| format!("Could not parse a Codex usage update: {error}"))?;
        apply_bucket(app, state, params.rate_limits).await;
    }

    Ok(())
}

fn select_main_bucket(mut response: RateLimitsReadResult) -> Option<RateLimitBucket> {
    response
        .rate_limits
        .or_else(|| response.rate_limits_by_limit_id.remove("codex"))
        .or_else(|| response.rate_limits_by_limit_id.into_values().next())
}

async fn apply_bucket(app: &AppHandle, state: &SharedUsageState, bucket: RateLimitBucket) {
    update_state(app, state, move |snapshot| snapshot.apply_bucket(bucket)).await;
}

async fn update_state(
    app: &AppHandle,
    state: &SharedUsageState,
    update: impl FnOnce(&mut UsageSnapshot),
) {
    let snapshot = {
        let mut snapshot = state.0.write().await;
        update(&mut snapshot);
        snapshot.clone()
    };
    let _ = app.emit(EVENT_NAME, snapshot);
}

async fn write_message(
    stdin: &mut tokio::process::ChildStdin,
    value: &Value,
) -> Result<(), String> {
    let mut message = serde_json::to_vec(value)
        .map_err(|error| format!("Could not serialize an App Server request: {error}"))?;
    message.push(b'\n');
    stdin
        .write_all(&message)
        .await
        .map_err(|error| format!("Could not write to Codex App Server: {error}"))?;
    stdin
        .flush()
        .await
        .map_err(|error| format!("Could not flush Codex App Server input: {error}"))
}

fn find_codex_binary() -> PathBuf {
    if let Some(path) = env::var_os("CODEX_USAGE_DOCK_CODEX_BIN") {
        return PathBuf::from(path);
    }

    #[cfg(target_os = "macos")]
    {
        for candidate in [
            "/Applications/ChatGPT.app/Contents/Resources/codex",
            "/Applications/Codex.app/Contents/Resources/codex",
        ] {
            let path = PathBuf::from(candidate);
            if path.exists() {
                return path;
            }
        }
    }

    #[cfg(target_os = "windows")]
    if let Some(local_app_data) = env::var_os("LOCALAPPDATA") {
        let root = PathBuf::from(local_app_data);
        for relative in [
            ["Programs", "ChatGPT", "resources", "codex.exe"].as_slice(),
            ["Programs", "ChatGPT", "codex.exe"].as_slice(),
            ["OpenAI", "ChatGPT", "resources", "codex.exe"].as_slice(),
        ] {
            let mut path = root.clone();
            path.extend(relative);
            if path.exists() {
                return path;
            }
        }
    }

    PathBuf::from(if cfg!(target_os = "windows") {
        "codex.exe"
    } else {
        "codex"
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_the_main_codex_bucket() {
        let primary = RateLimitBucket::default();
        let fallback = RateLimitBucket::default();
        let response = RateLimitsReadResult {
            rate_limits: Some(primary),
            rate_limits_by_limit_id: HashMap::from([("codex_other".into(), fallback)]),
        };

        assert!(select_main_bucket(response).is_some());
    }

    #[test]
    fn supports_the_multi_bucket_shape() {
        let response = RateLimitsReadResult {
            rate_limits: None,
            rate_limits_by_limit_id: HashMap::from([("codex".into(), RateLimitBucket::default())]),
        };

        assert!(select_main_bucket(response).is_some());
    }
}
