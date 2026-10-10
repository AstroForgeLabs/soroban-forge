//! forge-webhook-relay  (event pipeline)
//!
//! Connects to a Stellar Horizon Server-Sent Events (SSE) stream, relays each
//! contract event to a configured HTTP webhook, and signs every request body
//! with HMAC-SHA256 so the receiver can verify authenticity.
//!
//! # Configuration (environment variables)
//!
//! | Variable          | Required | Description                                                        |
//! |-------------------|----------|--------------------------------------------------------------------|
//! | `RELAY_STREAM_URL`| yes*     | Full SSE endpoint, e.g. `https://horizon-testnet.stellar.org/events`|
//! | `HORIZON_URL`     | no       | Used to derive the stream URL when `RELAY_STREAM_URL` is unset      |
//! | `WEBHOOK_URL`     | yes      | Destination that receives a `POST` per event                       |
//! | `WEBHOOK_SECRET`  | no       | When set, signs bodies with HMAC-SHA256                           |
//! | `RELAY_CURSOR`    | no       | Stream cursor to resume from (default: `now`)                      |
//! | `RUST_LOG`        | no       | Tracing filter (default: `forge_webhook_relay=info`)               |
//!
//! Each delivery includes the headers `X-Forge-Event` and, when a secret is
//! configured, `X-Forge-Signature: sha256=<hex>` over the exact request body.

use std::{env, time::Duration};

use futures_util::StreamExt;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use tokio::time::sleep;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

type HmacSha256 = Hmac<Sha256>;

const SIGNATURE_HEADER: &str = "x-forge-signature";
const EVENT_HEADER: &str = "x-forge-event";

#[derive(Clone, Debug)]
struct Config {
    stream_url: String,
    webhook_url: String,
    webhook_secret: Option<String>,
    cursor: String,
}

impl Config {
    fn from_env() -> Result<Self, String> {
        let stream_url = match env::var("RELAY_STREAM_URL") {
            Ok(url) => url,
            Err(_) => {
                let horizon = env::var("HORIZON_URL")
                    .map_err(|_| "set RELAY_STREAM_URL (or HORIZON_URL) to an SSE endpoint".to_string())?;
                format!("{}/events", horizon.trim_end_matches('/'))
            }
        };

        let webhook_url =
            env::var("WEBHOOK_URL").map_err(|_| "WEBHOOK_URL is required".to_string())?;

        let webhook_secret = env::var("WEBHOOK_SECRET").ok().filter(|s| !s.is_empty());

        let cursor = env::var("RELAY_CURSOR").unwrap_or_else(|_| "now".to_string());

        Ok(Self {
            stream_url,
            webhook_url,
            webhook_secret,
            cursor,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("forge_webhook_relay=info")),
        )
        .init();

    let config = Config::from_env()?;
    info!(
        stream = %config.stream_url,
        webhook = %config.webhook_url,
        signed = config.webhook_secret.is_some(),
        "starting forge-webhook-relay"
    );

    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .build()?;

    let mut backoff = Duration::from_secs(1);
    loop {
        match run_stream(&client, &config).await {
            Ok(()) => {
                info!("event stream closed, reconnecting");
                backoff = Duration::from_secs(1);
            }
            Err(err) => {
                error!(error = %err, "event stream error, retrying");
            }
        }

        sleep(backoff).await;
        backoff = (backoff * 2).min(Duration::from_secs(60));
    }
}

/// Open the SSE stream and dispatch every complete event until the stream ends.
async fn run_stream(
    client: &reqwest::Client,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = with_cursor(&config.stream_url, &config.cursor);
    let response = client
        .get(&url)
        .header("accept", "text/event-stream")
        .header("cache-control", "no-cache")
        .send()
        .await?
        .error_for_status()?;

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut event_data = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(newline) = buffer.find('\n') {
            let line = buffer[..newline].trim_end_matches('\r').to_string();
            buffer.drain(..=newline);

            if line.is_empty() {
                if !event_data.is_empty() {
                    let payload = std::mem::take(&mut event_data);
                    dispatch(client, config, payload.trim_end()).await;
                }
                continue;
            }

            if let Some(rest) = line.strip_prefix("data:") {
                if !event_data.is_empty() {
                    event_data.push('\n');
                }
                event_data.push_str(rest.strip_prefix(' ').unwrap_or(rest));
            }
        }
    }

    Ok(())
}

/// POST a single event payload to the configured webhook, signing it if needed.
async fn dispatch(client: &reqwest::Client, config: &Config, payload: &str) {
    if serde_json::from_str::<serde_json::Value>(payload).is_err() {
        warn!("skipping SSE payload that is not valid JSON");
        return;
    }

    let mut request = client
        .post(&config.webhook_url)
        .header("content-type", "application/json")
        .header(EVENT_HEADER, "contract-event");

    if let Some(secret) = &config.webhook_secret {
        let signature = sign(secret, payload.as_bytes());
        request = request.header(SIGNATURE_HEADER, format!("sha256={signature}"));
    }

    match request.body(payload.to_string()).send().await {
        Ok(response) if response.status().is_success() => {
            info!(status = %response.status(), "event delivered");
        }
        Ok(response) => {
            warn!(status = %response.status(), "webhook rejected event");
        }
        Err(err) => {
            error!(error = %err, "failed to deliver event");
        }
    }
}

/// Compute the hex-encoded HMAC-SHA256 of `body` using `secret`.
fn sign(secret: &str, body: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts keys of any length");
    mac.update(body);
    hex::encode(mac.finalize().into_bytes())
}

/// Append the cursor query parameter unless the URL already specifies one.
fn with_cursor(url: &str, cursor: &str) -> String {
    if url.contains("cursor=") {
        return url.to_string();
    }
    let separator = if url.contains('?') { '&' } else { '?' };
    format!("{url}{separator}cursor={cursor}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_is_deterministic_hex() {
        let signature = sign("secret", b"{\"amount\":10}");
        assert_eq!(signature.len(), 64);
        assert!(signature.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(signature, sign("secret", b"{\"amount\":10}"));
    }

    #[test]
    fn signature_changes_with_secret() {
        assert_ne!(sign("a", b"payload"), sign("b", b"payload"));
    }

    #[test]
    fn cursor_appended_when_missing() {
        assert_eq!(
            with_cursor("https://horizon.test/events", "now"),
            "https://horizon.test/events?cursor=now"
        );
        assert_eq!(
            with_cursor("https://horizon.test/events?order=asc", "now"),
            "https://horizon.test/events?order=asc&cursor=now"
        );
    }

    #[test]
    fn cursor_kept_when_present() {
        let url = "https://horizon.test/events?cursor=123";
        assert_eq!(with_cursor(url, "now"), url);
    }
}
