use std::str::FromStr;

use chrono::Datelike;
use clap::Parser;
use secrecy::{Secret, ExposeSecret};

use crate::caju::CajuClient;
use crate::ofx::Ofx;

mod caju;
mod ofx;

#[derive(Parser)]
struct App {
    #[arg(long = "base-url", env = "BASE_URL", default_value = "https://apigw.caju.com.br")]
    // Base url of the Caju API.
    base_url: String,

    #[arg(long = "bearer-token", env = "BEARER_TOKEN")]
    /// Bearer token for the Caju API. Can be obtained from a MITM proxy when opening the Caju
    /// mobile app.
    bearer_token: Secret<String>,

    #[arg(long = "refresh-token", env = "REFRESH_TOKEN")]
    /// Refresh token for the Caju API. Can be obtained from a MITM proxy when opening the Caju
    /// mobile app.
    refresh_token: Secret<String>,

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;

    let app = App::parse();

    let month = try_into_month(&app.month).unwrap_or_else(|_| {
        chrono::Month::try_from(chrono::Local::now().month() as u8)
            .expect("month from Local::now() should be valid")
    });
    let year = app.year.unwrap_or_else(|| chrono::Local::now().year());

    let mut client = CajuClient::new(app.base_url, app.user_id, app.employee_id)?;
    client.login(app.bearer_token.expose_secret(), app.refresh_token.expose_secret()).await?;
    let client = client;

    let statement = client.get_month_statement(Some(year), month).await?;

    let ofx: Ofx = match statement.try_into() {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Error for {}/{}: {}", month.name(), year, e);
            return Err(e);
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
