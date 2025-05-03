use secrecy::SecretString;

use self::auth::{AuthState, FlashAuthentication};

pub mod auth;
pub mod statement;

pub struct FlashClient {
    employee_id: String,
    company_id: String,
    username: String,
    password: SecretString,
    client: reqwest::Client,
    auth: auth::AuthState,
}

impl FlashClient {
    pub fn new(
        username: String,
        password: SecretString,
        company_id: String,
        employee_id: String,
    ) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:135.0) Gecko/20100101 Firefox/135.0")
            .build()
            .expect("reqwest client should have been build");
        Self {
            username,
            password,
            client,
            auth: AuthState::NotStarted,
            company_id,
            employee_id,
        }
    }

    pub fn auth_override(
        auth: FlashAuthentication,
        company_id: String,
        employee_id: String,
    ) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:135.0) Gecko/20100101 Firefox/135.0")
            .build()
            .expect("reqwest client should have been build");
        Self {
            username: Default::default(),
            password: Default::default(),
            client,
            auth: AuthState::Authenticated(auth),
            company_id,
            employee_id,
        }
    }
}
