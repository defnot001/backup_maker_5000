use std::path::Path;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use anyhow::Context;
use reqwest::Client;

#[derive(Debug, Serialize)]
struct Claims {
    iss: String,
    scope: String,
    aud: String,
    exp: usize,
    iat: usize,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: usize,
}

#[derive(Debug, Deserialize)]
struct GCSCredentials {
    private_key: String,
    client_email: String,
    token_uri: String,
}

impl GCSCredentials {
    fn load_from_file(credentials_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let credentials_string = std::fs::read_to_string(credentials_path)
            .context("Failed to read GCS credentials file")?;
        serde_json::from_str(&credentials_string)
            .context("Failed to parse GCS credentials file")
    }

    fn private_key(&self) -> &str {
        &self.private_key
    }
}


pub async fn get_access_token(credentials_path: impl AsRef<Path>) -> anyhow::Result<String> {
    let credentials = GCSCredentials::load_from_file(credentials_path)
        .context("Failed to load GCS credentials")?;

    let issued_at = chrono::Utc::now().timestamp() as usize;
    let expiration = issued_at + 3600; // Token expires in one hour

    let claims = Claims {
        iss: credentials.client_email.clone(),
        scope: "https://www.googleapis.com/auth/devstorage.read_write".to_string(),
        aud: credentials.token_uri.clone(),
        exp: expiration,
        iat: issued_at,
    };

    let encoding_key = EncodingKey::from_rsa_pem(credentials.private_key().as_bytes())
        .context("Failed to create encoding key from private key")?;
    let header = Header::new(Algorithm::RS256);
    let jwt = jsonwebtoken::encode(&header, &claims, &encoding_key)
        .context("Failed to encode JWT")?;

    let client = Client::new();
    let params = [
        ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
        ("assertion", &jwt),
    ];

    let response = client.post(&credentials.token_uri)
        .form(&params)
        .send()
        .await
        .context("Failed to send token request to Google OAuth2 endpoint")?
        .json::<TokenResponse>()
        .await
        .context("Failed to decode response from Google OAuth2 endpoint")?;

    Ok(response.access_token)
}