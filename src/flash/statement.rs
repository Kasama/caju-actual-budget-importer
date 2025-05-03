use chrono::{Datelike, Months, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use serde::Deserializer;
use serde_json::json;

use crate::flash::auth::AuthState;
use crate::ofx::{
    Ofx, OfxBankAccount, OfxCreditCard, OfxCreditCardStatement, OfxStatement, OfxStatementStatus,
    OfxTransactions,
};

use super::FlashClient;

const FLASH_BFF_URL: &str = "https://corporate-card-bff.us.flashapp.services/bff/trpc";

fn serialize_naive_date_time<S>(date: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let formatted_date = date.format("%Y-%m-%dT%H:%M:%S.%3fZ").to_string();
    serializer.serialize_str(&formatted_date)
}

struct NaiveDateTimeVisitor;
impl serde::de::Visitor<'_> for NaiveDateTimeVisitor {
    type Value = NaiveDateTime;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a string that represents a date-time")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match NaiveDateTime::parse_from_str(v, "%Y-%m-%dT%H:%M:%S.%fZ") {
            Ok(t) => Ok(t),
            Err(_) => Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(v),
                &self,
            )),
        }
    }
}

fn from_timestamp<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_str(NaiveDateTimeVisitor)
}

#[derive(Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FlashTransactionStatus {
    Completed,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionType {
    Deposit,
    OpenLoopPayment,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlashTransaction {
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(deserialize_with = "from_timestamp")]
    pub date: NaiveDateTime,
    pub amount: u64,
    pub description: String,
    pub status: FlashTransactionStatus,
    #[serde(rename = "type")]
    pub type_: TransactionType,
}

impl TryFrom<Vec<FlashTransaction>> for Ofx {
    type Error = anyhow::Error;

    fn try_from(value: Vec<FlashTransaction>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(anyhow::anyhow!("No statement to convert"));
        }
        let start = value.first().unwrap().date;
        let end = value.last().unwrap().date;

        Ok(Ofx {
            bank: None,
            credit_card: Some(OfxCreditCard {
                statement: OfxCreditCardStatement {
                    transaction_id: "transaction_id".to_string(),
                    status: OfxStatementStatus {
                        code: 0,
                        severity: "INFO".to_string(),
                    },
                    statements: OfxStatement {
                        currency_code: "BRL".to_string(),
                        bank_account: OfxBankAccount {
                            bank_id: "Flash".to_string(),
                        },
                        transactions: OfxTransactions {
                            start: start.format("%Y%m%d000000[-3:BRT]").to_string(),
                            end: end.format("%Y%m%d000000[-3:BRT]").to_string(),
                            transactions: value
                                .into_iter()
                                .filter(|transaction| {
                                    transaction.status == FlashTransactionStatus::Completed
                                })
                                .map(|transaction| {
                                    crate::ofx::OfxTransactionVariant::Transaction(
                                        crate::ofx::OfxTransaction {
                                            type_: match transaction.type_ {
                                                TransactionType::Deposit => "CREDIT",
                                                TransactionType::OpenLoopPayment => "DEBIT",
                                            }
                                            .to_string(),
                                            timestamp: transaction
                                                .date
                                                .format("%Y%m%d000000[-3:BRT]")
                                                .to_string(),
                                            amount: format!(
                                                "{:.2}",
                                                (transaction.amount as f64) / 100.0
                                                    * (match transaction.type_ {
                                                        TransactionType::Deposit => 1.0,
                                                        TransactionType::OpenLoopPayment => -1.0,
                                                    })
                                            ),
                                            id: transaction.id,
                                            description: transaction.description,
                                        },
                                    )
                                })
                                .collect(),
                        },
                    },
                },
            }),
        })
    }
}

impl FlashClient {
    pub async fn get_month_statement(
        &self,
        year: Option<i32>,
        month: chrono::Month,
    ) -> anyhow::Result<Vec<FlashTransaction>> {
        let auth = match &self.auth {
            AuthState::Authenticated(auth) => auth,
            _ => anyhow::bail!("Not authenticated"),
        };

        // Request structs
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct QueryPagination {
            current_page: u32,
            page_size: u32,
        }

        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct QueryFilter {
            #[serde(serialize_with = "serialize_naive_date_time")]
            start_date: NaiveDateTime,
            #[serde(serialize_with = "serialize_naive_date_time")]
            end_date: NaiveDateTime,
        }

        // Response structs
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Response {
            result: ResponseResult,
        }

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ResponseResult {
            data: ResponseData,
        }

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ResponseData {
            json: ResponseJson,
        }

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ResponseJson {
            items: Vec<FlashTransaction>,
            meta: ResponsePagination,
        }

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ResponsePagination {
            current_page: u32,
            total_items: u32,
            total_pages: u32,
            page_size: u32,
        }

        let pagination = QueryPagination {
            current_page: 0,
            page_size: 100,
        };

        let first_day_of_month = NaiveDate::from_ymd_opt(
            year.unwrap_or_else(|| chrono::Local::now().year()),
            month.number_from_month(),
            1,
        )
        .ok_or(anyhow::anyhow!("Failed to get current month"))?
        .and_time(NaiveTime::from_hms_opt(3, 0, 0).unwrap());

        let last_day_of_month = first_day_of_month
            .checked_add_months(Months::new(1))
            .ok_or(anyhow::anyhow!("Failed to add a month to current month"))?
            .with_hour(23)
            .unwrap()
            .with_minute(59)
            .unwrap()
            .with_second(59)
            .unwrap();

        let filter = QueryFilter {
            start_date: first_day_of_month,
            end_date: last_day_of_month,
        };

        let meta = json!({
            "values": {
                "filter.endDate": ["Date"],
                "filter.startDate": ["Date"]
            }
        });

        let statement_request_query = json!({
            "0": {
                "json": {
                    "pagination": pagination,
                    "filter": filter
                },
                "meta": meta
            }
        });

        let resp = self
            .client
            .get(format!("{}/person.getStatement", FLASH_BFF_URL))
            .query(&[
                ("batch", "1"),
                ("input", &statement_request_query.to_string()),
            ])
            .header("Authorization", &auth.token)
            .header("x-flash-auth", format!("Bearer {}", auth.token))
            .header("company-id", &self.company_id)
            .send()
            .await?;

        let resp_text = resp.text().await?;
        println!("statement response: {:?}", resp_text);
        let mut resp: Vec<Response> = serde_json::from_str(&resp_text)?;

        let items = match resp.pop() {
            Some(r) => r.result.data.json.items,
            _ => vec![],
        };

        Ok(items)
    }
}
