//! OAuth2 PKCE authentication for Berget AI.
//!
//! This module implements the OAuth2 Authorization Code flow with PKCE (Proof Key for Code Exchange)
//! for authenticating with Berget AI's Keycloak instance.

use anyhow::{Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use oauth2::AuthorizationCode;
use rand::Rng;
use sha2::{Digest, Sha256};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use url::Url;

/// Keycloak configuration
const KEYCLOAK_URL: &str = "https://keycloak.berget.ai";
const KEYCLOAK_REALM: &str = "berget";
const KEYCLOAK_CLIENT_ID: &str = "berget-code";
const CALLBACK_PORT: u16 = 8787;

/// OAuth2 authentication result
#[derive(Debug)]
pub struct OAuthResult {
    pub access_token: String,
    pub refresh_token: Option<String>,
}

fn generate_code_verifier() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn generate_code_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let result = hasher.finalize();
    URL_SAFE_NO_PAD.encode(result)
}

fn generate_state() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill(&mut bytes);
    hex::encode(bytes)
}

fn build_authorization_url(code_challenge: &str, state: &str, redirect_uri: &str) -> Result<Url> {
    let auth_url = format!("{}/realms/{}/protocol/openid-connect/auth", KEYCLOAK_URL, KEYCLOAK_REALM);
    let mut url = Url::parse(&auth_url).context("Failed to parse Keycloak authorization URL")?;
    url.query_pairs_mut()
        .append_pair("client_id", KEYCLOAK_CLIENT_ID)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("scope", "openid email profile")
        .append_pair("state", state)
        .append_pair("code_challenge", code_challenge)
        .append_pair("code_challenge_method", "S256");
    Ok(url)
}

async fn start_callback_server(expected_state: String) -> Result<oneshot::Receiver<Result<AuthorizationCode>>> {
    let addr: SocketAddr = format!("127.0.0.1:{}", CALLBACK_PORT)
        .parse()
        .context("Failed to parse callback server address")?;
    let listener = TcpListener::bind(&addr)
        .await
        .context("Failed to bind callback server")?;
    tracing::debug!("Callback server listening on {}", addr);
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        if let Ok((mut stream, _)) = listener.accept().await {
            use tokio::io::AsyncReadExt;
            let mut buffer = [0; 4096];
            if let Ok(n) = stream.read(&mut buffer).await {
                let request = String::from_utf8_lossy(&buffer[..n]);
                let code = if let Some(line) = request.lines().next() {
                    if let Some(path) = line.split_whitespace().nth(1) {
                        let url = format!("http://localhost:{}{}", CALLBACK_PORT, path);
                         match Url::parse(&url) {
                             Ok(parsed) => {
                                 let received_state: Option<String> = parsed
                                     .query_pairs()
                                     .find(|(k, _)| k.as_ref() == "state")
                                     .map(|(_, v)| v.to_string());
                                 if received_state.as_deref() != Some(&expected_state) {
                                     send_html_response(&mut stream, "Authentication Failed", "Invalid state parameter. Please try again.", false).await;
                                     let _ = tx.send(Err(anyhow::anyhow!("Invalid state parameter")));
                                     return;
                                 }
                                 parsed.query_pairs()
                                     .find(|(k, _)| k.as_ref() == "code")
                                     .map(|(_, v)| AuthorizationCode::new(v.to_string()))
                             }
                             Err(_) => None,
                         }
                    } else { None }
                } else { None };
                match code {
                    Some(auth_code) => {
                        send_html_response(&mut stream, "Authentication Successful", "You can close this window and return to your terminal.", true).await;
                        let _ = tx.send(Ok(auth_code));
                    }
                     None => {
                         let error: Option<String> = if let Some(line) = request.lines().next() {
                             if let Some(path) = line.split_whitespace().nth(1) {
                                 let url = format!("http://localhost:{}{}", CALLBACK_PORT, path);
                                 match Url::parse(&url) {
                                     Ok(parsed) => parsed.query_pairs().find(|(k, _)| k.as_ref() == "error").map(|(_, v)| v.to_string()),
                                     Err(_) => None,
                                 }
                             } else { None }
                         } else { None };
                        if let Some(err) = &error {
                            send_html_response(&mut stream, "Authentication Failed", err, false).await;
                            let _ = tx.send(Err(anyhow::anyhow!("Authentication failed: {}", err)));
                        } else {
                            send_html_response(&mut stream, "Authentication Failed", "No authorization code received", false).await;
                            let _ = tx.send(Err(anyhow::anyhow!("No authorization code received")));
                        }
                    }
                }
            }
        }
    });
    Ok(rx)
}

async fn send_html_response(stream: &mut tokio::net::TcpStream, title: &str, message: &str, success: bool) {
    use tokio::io::AsyncWriteExt;
    let color = if success { "#22c55e" } else { "#ef4444" };
    let icon = if success {
        r#"<polyline points="20 6 9 17 4 12"></polyline>"#
    } else {
        r#"<line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line>"#
    };
    let html = format!(
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><title>Berget - {}</title><style>*{{margin:0;padding:0;box-sizing:border-box}}body{{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Oxygen,Ubuntu,sans-serif;display:flex;justify-content:center;align-items:center;min-height:100vh;background:linear-gradient(135deg,#0f0f1a 0%,#1a1a2e 50%,#16213e 100%);color:#fff}}.container{{text-align:center;padding:3rem;max-width:400px}}.icon{{width:80px;height:80px;background:linear-gradient(135deg,{} 0%,{} 100%);border-radius:50%;display:flex;align-items:center;justify-content:center;margin:0 auto 1.5rem;box-shadow:0 4px 20px rgba(34,197,94,0.3)}}.icon svg{{width:40px;height:40px;stroke:#fff;stroke-width:3}}h1{{font-size:1.5rem;font-weight:600;margin-bottom:0.75rem;color:#fff}}p{{color:#94a3b8;font-size:0.95rem;line-height:1.5}}.brand{{margin-top:2rem;opacity:0.5;font-size:0.8rem;letter-spacing:0.05em}}</style></head><body><div class="container"><div class="icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor">{}</svg></div><h1>{}</h1><p>{}</p><div class="brand">BERGET</div></div></body></html>"#,
        title, color, color, icon, title, message
    );
    let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\r\n{}", html.len(), html);
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.flush().await;
}

fn open_browser(url: &Url) -> Result<()> {
    #[cfg(target_os = "macos")]
    { std::process::Command::new("open").arg(url.as_str()).spawn().context("Failed to open browser")?; }
    #[cfg(target_os = "linux")]
    {
        if let Ok(output) = std::process::Command::new("xdg-open").arg(url.as_str()).output() {
            if !output.status.success() { anyhow::bail!("Failed to open browser"); }
        } else { anyhow::bail!("Failed to open browser: xdg-open not found"); }
    }
    #[cfg(target_os = "windows")]
    { std::process::Command::new("cmd").args(["/C", "start", "", url.as_str()]).spawn().context("Failed to open browser")?; }
    Ok(())
}

async fn exchange_code_for_token(code: AuthorizationCode, code_verifier: &str, redirect_uri: &str) -> Result<OAuthResult> {
    let token_url = format!("{}/realms/{}/protocol/openid-connect/token", KEYCLOAK_URL, KEYCLOAK_REALM);
    let client = reqwest::Client::new();
    let params = [("grant_type", "authorization_code"), ("client_id", KEYCLOAK_CLIENT_ID), ("code", code.secret()), ("redirect_uri", redirect_uri), ("code_verifier", code_verifier)];
    let response = client.post(&token_url).form(&params).send().await.context("Failed to exchange authorization code for token")?;
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("Failed to exchange code for token (status {}): {}", status, error_text);
    }
    #[derive(serde::Deserialize)]
    struct TokenResponse { access_token: String, refresh_token: Option<String>, expires_in: u64 }
    let token_data: TokenResponse = response.json().await.context("Failed to parse token response")?;
    tracing::debug!("Token received: expires_in={} seconds", token_data.expires_in);
    Ok(OAuthResult { access_token: token_data.access_token, refresh_token: token_data.refresh_token })
}

pub async fn authenticate_with_pkce() -> Result<OAuthResult> {
    tracing::info!("Initiating OAuth2 PKCE authentication flow");
    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);
    let state = generate_state();
    tracing::debug!("Generated PKCE parameters");
    let redirect_uri = format!("http://localhost:{}/callback", CALLBACK_PORT);
    let auth_url = build_authorization_url(&code_challenge, &state, &redirect_uri)?;
    tracing::debug!("Authorization URL built: {}", auth_url);
    let callback_rx = start_callback_server(state).await?;
    if let Err(e) = open_browser(&auth_url) {
        tracing::warn!("Failed to open browser: {}", e);
        println!("\nPlease open this URL in your browser:");
        println!("{}", auth_url);
    } else {
        println!("\nBrowser opened for authentication...");
    }
    let timeout = Duration::from_secs(5 * 60);
    let auth_code = tokio::time::timeout(timeout, callback_rx).await.context("Authentication timed out after 5 minutes")?.context("Callback server closed unexpectedly")??;
    tracing::debug!("Authorization code received");
    let oauth_result = exchange_code_for_token(auth_code, &code_verifier, &redirect_uri).await?;
    tracing::info!("OAuth2 PKCE authentication successful");
    Ok(oauth_result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_code_verifier() {
        let verifier = generate_code_verifier();
        assert!(!verifier.is_empty());
        assert!(verifier.len() >= 32);
        assert!(!verifier.contains('='));
        assert!(!verifier.contains('+'));
        assert!(!verifier.contains('/'));
    }

    #[test]
    fn test_generate_code_challenge() {
        let verifier = generate_code_verifier();
        let challenge = generate_code_challenge(&verifier);
        assert!(!challenge.is_empty());
        assert!(challenge.len() >= 32);
        assert!(!challenge.contains('='));
        assert!(!challenge.contains('+'));
        assert!(!challenge.contains('/'));
        assert_ne!(verifier, challenge);
    }

    #[test]
    fn test_generate_state() {
        let state = generate_state();
        assert!(!state.is_empty());
        assert!(state.len() == 32);
        assert!(state.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_build_authorization_url() {
        let code_challenge = "test_challenge";
        let state = "test_state";
        let redirect_uri = "http://localhost:8787/callback";
        let url = build_authorization_url(code_challenge, state, redirect_uri).unwrap();
        assert_eq!(url.scheme(), "https");
        assert!(url.host_str().unwrap().contains("keycloak"));
        assert!(url.path().contains("auth"));
        let query_pairs: Vec<(String, String)> = url.query_pairs().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        assert!(query_pairs.iter().any(|(k, v)| k == "client_id" && v == "berget-code"));
        assert!(query_pairs.iter().any(|(k, v)| k == "response_type" && v == "code"));
        assert!(query_pairs.iter().any(|(k, v)| k == "redirect_uri" && v == redirect_uri));
        assert!(query_pairs.iter().any(|(k, v)| k == "state" && v == state));
        assert!(query_pairs.iter().any(|(k, v)| k == "code_challenge" && v == code_challenge));
        assert!(query_pairs.iter().any(|(k, v)| k == "code_challenge_method" && v == "S256"));
    }
}
