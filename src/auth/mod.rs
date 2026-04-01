//! Authentication module for ostt.
//!
//! Provides OAuth2 PKCE authentication for Berget AI.

pub mod berget_api;
pub mod oauth2;

pub use berget_api::create_api_key;
pub use oauth2::OAuthResult;
pub use oauth2::authenticate_with_pkce;
