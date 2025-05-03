use std::io::{BufRead, Write};
use std::str::FromStr;

use chrono::Datelike;
use clap::Parser;
use secrecy::{ExposeSecret, SecretString};

use crate::caju::CajuClient;
use crate::ofx::Ofx;

use self::flash::FlashClient;

mod caju;
mod flash;
mod ofx;

#[derive(Parser)]
struct App {
    #[arg(
        long = "base-url",
        env = "BASE_URL",
        default_value = "https://apigw.caju.com.br"
    )]
    // Base url of the Caju API.
    base_url: String,

    #[arg(long = "bearer-token", env = "BEARER_TOKEN")]
    /// Bearer token for the Caju API. Can be obtained from a MITM proxy when opening the Caju
    /// mobile app.
    bearer_token: SecretString,

    #[arg(long = "refresh-token", env = "REFRESH_TOKEN")]
    /// Refresh token for the Caju API. Can be obtained from a MITM proxy when opening the Caju
    /// mobile app.
    refresh_token: SecretString,

    #[arg(long = "flash-username", env = "FLASH_USERNAME")]
    flash_username: String,

    #[arg(long = "flash-password", env = "FLASH_PASSWORD")]
    flash_password: SecretString,

    #[arg(long = "flash-override-token", env = "FLASH_AUTH_OVERRIDE_TOKEN")]
    flash_override_token: Option<String>,

    #[arg(long = "flash-company", env = "FLASH_COMPANY_ID")]
    flash_company_id: String,

    #[arg(long = "user-id", env = "USER_ID")]
    // User id of your caju user. Can be obtained from a MITM proxy when opening the Caju app.
    user_id: String,

    #[arg(long = "employee-id", env = "EMPLOYEE_ID")]
    // Employee id of your caju account. Can be obtained from a MITM proxy when opening the Caju app.
    employee_id: String,

    /// Month to get statement for. Accepts numbers or english month names.
    month: String,

    /// Year to get statement for. Default is current year according to local timezone.
    year: Option<i32>,

    #[arg(short = 'o', long = "output")]
    /// The file name to output OFX to. Default is stdout.
    filename: Option<String>,
}

enum Providers {
    Flash,
    Caju,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;

    let app = App::parse();

    let month = try_into_month(&app.month).unwrap_or_else(|_| {
        chrono::Month::try_from(chrono::Local::now().month() as u8)
            .expect("month from Local::now() should be valid")
    });
    let year = app.year.unwrap_or_else(|| chrono::Local::now().year());

    let provider = Providers::Flash;

    let ofx: Ofx = match provider {
        Providers::Flash => {
            let client = match app.flash_override_token {
                Some(token) => {
                    println!("Using override token");
                    FlashClient::auth_override(
                        flash::auth::FlashAuthentication { token },
                        app.flash_company_id,
                        app.employee_id,
                    )
                }
                _ => {
                    let mut client = FlashClient::new(
                        app.flash_username.to_string(),
                        app.flash_password.clone(),
                        app.flash_company_id,
                        app.employee_id,
                    );

                    client.initiate_auth().await?;

                    print!("Enter TOTP: ");
                    std::io::stdout().flush()?;

                    let totp = {
                        let stdin = std::io::stdin().lock();
                        let line = stdin.lines().next().ok_or(anyhow::anyhow!("no input"))??;
                        line.trim().to_string()
                    };

                    client.finish_login(&totp).await?;

                    client
                }
            };

            client
                .get_month_statement(Some(year), month)
                .await?
                .try_into()?
        }
        Providers::Caju => {
            let mut client = CajuClient::new(app.base_url, app.user_id, app.employee_id)?;
            client
                .login(
                    app.bearer_token.expose_secret(),
                    app.refresh_token.expose_secret(),
                )
                .await?;
            let client = client;

            client
                .get_month_statement(Some(year), month)
                .await?
                .try_into()?
        }
    };

    match app.filename {
        Some(ref fname) => Box::new(
            std::fs::OpenOptions::new()
                .truncate(true)
                .create(true)
                .write(true)
                .open(fname)?,
        ) as Box<dyn std::io::Write>,
        None => Box::new(std::io::stdout()) as Box<dyn std::io::Write>,
    }
    .write_all(ofx.to_ofx()?.as_bytes())?;

    if let Some(ref filename) = app.filename {
        println!("Wrote ofx for {}/{} at {}", month.name(), year, filename);
    }

    Ok(())
}

fn try_into_month(input: &str) -> anyhow::Result<chrono::Month> {
    let parsed = match chrono::Month::from_str(input) {
        Ok(m) => m,
        Err(_) => match input.parse::<u8>() {
            Ok(month_number) => chrono::Month::try_from(month_number)?,
            Err(e) => Err(e)?,
        },
    };

    Ok(parsed)
}

#[cfg(test)]
mod test {

    use crate::try_into_month;

    #[test]
    fn parse_months() -> Result<(), anyhow::Error> {
        let tests = ["1", "january", "January", "JANUARY", "JanUaRY"];

        let results = tests.map(try_into_month);

        println!("{:?}", results);

        results.into_iter().collect::<Result<Vec<_>, _>>()?;

        Ok(())
    }
}
