use std::time::Duration;

use clap::Parser;

use crate::caju::CajuClient;
use crate::ofx::Ofx;

mod caju;
mod ofx;

#[derive(Parser)]
struct App {
    #[arg(env = "BEARER_TOKEN")]
    bearer_token: String,
    #[arg(env = "REFRESH_TOKEN")]
    refresh_token: String,
    #[arg(env = "BASE_URL")]
    base_url: String,
    #[arg(env = "USER_ID")]
    user_id: String,
    #[arg(env = "EMPLOYEE_ID")]
    employee_id: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;

    let app = App::parse();

    let mut client = CajuClient::new(app.base_url, app.user_id, app.employee_id)?;
    client.login(app.bearer_token, app.refresh_token).await?;
    let client = client;

    let months = vec![
        (2021, chrono::Month::April),
        (2021, chrono::Month::May),
        (2021, chrono::Month::June),
        (2021, chrono::Month::July),
        (2021, chrono::Month::August),
        (2021, chrono::Month::September),
        (2021, chrono::Month::October),
        (2021, chrono::Month::November),
        (2021, chrono::Month::December),
        (2022, chrono::Month::January),
        (2022, chrono::Month::February),
        (2022, chrono::Month::March),
        (2022, chrono::Month::April),
        (2022, chrono::Month::May),
        (2022, chrono::Month::June),
        (2022, chrono::Month::July),
        (2022, chrono::Month::August),
        (2022, chrono::Month::September),
        (2022, chrono::Month::October),
        (2022, chrono::Month::November),
        (2022, chrono::Month::December),
        (2023, chrono::Month::January),
        (2023, chrono::Month::February),
        (2023, chrono::Month::March),
        (2023, chrono::Month::April),
        (2023, chrono::Month::May),
        (2023, chrono::Month::June),
    ];

    for (year, month) in months {
        let statement = client.get_month_statement(Some(year), month).await?;

        let ofx: Ofx = match statement.try_into() {
            Ok(i) => i,
            Err(e) => {
                println!("Error for {}/{}: {}", month.name(), year, e);
                continue;
            },
        };

        let filename = format!("caju-{}-{}.ofx", year, month.name());

        std::fs::write(&filename, ofx.to_ofx()?)?;

        println!("Wrote ofx for {}", filename);

        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    Ok(())
}
