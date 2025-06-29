use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct RegisterInput {
    pub username: String,
    pub password_hash: String,
    pub icon: String,
}

#[derive(Serialize)]
pub struct LoginInput {
    pub username: String,
    pub password_hash: String,
}

#[derive(Deserialize)]
pub struct TokenResponse {
    pub token: String,
}

pub async fn register(
    username: &str,
    password: &str,
    icon: &str,
    api_base: &str,
) -> Result<TokenResponse, String> {
    let client = Client::new();
    let register_url = format!("{}/auth/register", api_base);
    let register_body = RegisterInput {
        username: username.to_string(),
        password_hash: password.to_string(),
        icon: icon.to_string(),
    };

    let res = client
        .post(&register_url)
        .json(&register_body)
        .send()
        .await
        .map_err(|e| format!("Register request failed: {e}"))?;

    if res.status().is_success() {
        res.json::<TokenResponse>()
            .await
            .map_err(|e| format!("Invalid registration response: {e}"))
    } else {
        let err = res.text().await.unwrap_or_default();
        Err(format!("Register failed: {}", err))
    }
}

pub async fn login(
    username: &str,
    password: &str,
    api_base: &str,
) -> Result<TokenResponse, String> {
    let client = Client::new();
    let login_url = format!("{}/auth/login", api_base);
    let login_body = LoginInput {
        username: username.to_string(),
        password_hash: password.to_string(),
    };

    let res = client
        .post(&login_url)
        .json(&login_body)
        .send()
        .await
        .map_err(|e| format!("Login request failed: {e}"))?;

    if res.status().is_success() {
        res.json::<TokenResponse>()
            .await
            .map_err(|e| format!("Invalid login response: {e}"))
    } else {
        let err = res.text().await.unwrap_or_default();
        Err(format!("Login failed: {}", err))
    }
}
