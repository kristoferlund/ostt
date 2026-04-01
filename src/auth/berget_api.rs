//! Berget API client for creating API keys.
//!
//! This module provides functions to interact with Berget's API for creating
//! and managing API keys using OAuth2 authentication.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const BERGET_API_BASE: &str = "https://api.berget.ai";

/// API key creation request
#[derive(Debug, Serialize)]
struct CreateApiKeyRequest {
    name: String,
    description: String,
}

/// API key response
#[derive(Debug, Deserialize)]
struct ApiKeyResponse {
    #[allow(dead_code)]
    id: String,
    key: String,
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    description: String,
}

/// Creates a new API key using the OAuth access token.
///
/// Makes a POST request to Berget's API key creation endpoint.
/// Returns the created API key.
pub async fn create_api_key(access_token: &str) -> Result<String> {
    let client = reqwest::Client::new();

    let request = CreateApiKeyRequest {
        name: "ostt-cli".to_string(),
        description: "API key created by ostt CLI via OAuth2 PKCE authentication".to_string(),
    };

    let url = format!("{}/v1/api-keys", BERGET_API_BASE);

    tracing::debug!("Creating API key at {}", url);

    let response = client
        .post(&url)
        .bearer_auth(access_token)
        .json(&request)
        .send()
        .await
        .context("Failed to create API key")?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!(
            "Failed to create API key (status {}): {}",
            status,
            error_text
        );
    }

    // Get response text for debugging
    let response_text = response.text().await
        .context("Failed to read API key response")?;

    tracing::debug!("API key response: {}", response_text);

    let api_key_response: ApiKeyResponse = serde_json::from_str(&response_text)
        .with_context(|| format!("Failed to parse API key response: {}", response_text))?;

    tracing::info!("API key created successfully: name={}", api_key_response.name);

    Ok(api_key_response.key)
}
