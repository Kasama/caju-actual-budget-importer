use secrecy::ExposeSecret;
use serde_json::json;

use super::FlashClient;

const FLASH_URL: &str = "https://corporate-card-bff.us.flashapp.services";
const FLASH_WEB_AUTH_URL: &str = "https://flashos-entrance.us.flashapp.services/v1/auth";
const AUTH_URL: &str = "https://hros-auth.flashapp.services";

const FLASH_CLIENT_ID: &str = "4r4ki1jqohppg2dko3uf7rvq13";

pub enum AuthState {
    NotStarted,
    Initialized(String),
    Authenticated(FlashAuthentication),
}

#[derive(serde::Deserialize)]
struct InitiateAuthResponse {
    // challenge_name: String,
    session: String,
}
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AuthenticationResult {
    access_token: String,
    expires_in: i64,
    token_type: String,
    refresh_token: String,
    id_token: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlashAuthentication {
    pub token: String,
}

impl FlashClient {
    pub async fn initiate_auth(&mut self) -> Result<(), anyhow::Error> {
        match self.auth {
            AuthState::NotStarted => (),
            _ => return Ok(()),
        }

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct InitiateAuthResponse {
            // challenge_name: String,
            session: String,
        }
        let response = self
            .client
            .post(AUTH_URL)
            .header(
                "X-Amz-Target",
                "AWSCognitoIdentityProviderService.InitiateAuth",
            )
            .body(
                json! ({
                    "AuthFlow": "USER_PASSWORD_AUTH",
                    "ClientId": FLASH_CLIENT_ID,
                    "AuthParameters": {
                        "USERNAME": self.username,
                        "PASSWORD": self.password.expose_secret()
                    },
                    "ClientMetadata": {
                        "preferredMfa": "SMS_MFA"
                    }
                })
                .to_string(),
            )
            .send()
            .await?;

        let value = response.text().await?;
        eprintln!("initate auth response: {:?}", value);
        let auth_initiate_response: InitiateAuthResponse = serde_json::from_str(&value)?;

        // let auth_initiate_response: InitiateAuthResponse = response.json().await?;

        self.auth = AuthState::Initialized(auth_initiate_response.session);

        Ok(())
    }

    pub async fn finish_login(&mut self, second_factor: &str) -> anyhow::Result<()> {
        let session = match self.auth {
            AuthState::NotStarted => {
                return Err(anyhow::anyhow!(
                    "auth not started. Call initiate_auth first"
                ))
            }
            AuthState::Initialized(ref session) => session,
            AuthState::Authenticated(_) => return Ok(()),
        };

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct RespondToAuthChallengeResponse {
            // challenge_parameters: serde_json::Value,
            authentication_result: AuthenticationResult,
        }
        let response = self
            .client
            .post(AUTH_URL)
            .header(
                "X-Amz-Target",
                "AWSCognitoIdentityProviderService.RespondToAuthChallenge",
            )
            .body(
                json! ({
                    "ChallengeName": "SMS_MFA",
                    "ChallengeResponses": {
                        "USERNAME": self.username,
                        "SMS_MFA_CODE": second_factor
                    },
                    "ClientId": FLASH_CLIENT_ID,
                    "Session": session
                })
                .to_string(),
            )
            .send()
            .await?;

        let value = response.text().await?;
        eprintln!("2fa auth response: {:?}", value);
        let auth_response: RespondToAuthChallengeResponse = serde_json::from_str(&value)?;

        // let auth_response: RespondToAuthChallengeResponse = response.json().await?;

        let token = auth_response.authentication_result.access_token.clone();

        eprintln!("got auth token: {}", token);

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct SignInEmployeeInnerResult {
            data: FlashAuthentication,
        }

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct SignInEmployeeResponse {
            result: SignInEmployeeInnerResult,
        }

        let signing_employee_response = self
            .client
            .post(format!("{}/trpc/signInEmployee", FLASH_WEB_AUTH_URL))
            .bearer_auth(token)
            .body(
                json!({
                    "employeeId":self.employee_id,
                    "companyId": self.company_id
                })
                .to_string(),
            )
            .send()
            .await?;

        let resp_text = signing_employee_response.text().await?;

        eprintln!("signing employee response: {:?}", resp_text);

        let resp: SignInEmployeeResponse = serde_json::from_str(&resp_text)?;

        let auth = resp.result.data;

        eprintln!("token: {:?}", auth.token);

        self.auth = AuthState::Authenticated(auth);

        Ok(())
    }
}
