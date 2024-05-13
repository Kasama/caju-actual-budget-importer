use std::write;

use chrono::{Datelike, Months, NaiveDate, NaiveDateTime};
use reqwest::header::HeaderMap;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::json;

use crate::ofx::{
    Ofx, OfxBankAccount, OfxCreditCard, OfxCreditCardStatement, OfxStatement, OfxStatementStatus,
    OfxTransactionVariant, OfxTransactions,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    bearer_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatementResponse {
    has_next: bool,
    items: Vec<StatementResponseItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatementResponseItem {
    cursor: Option<String>,
    item: StatementItem,
}

struct NaiveDateTimeVisitor;
impl<'de> serde::de::Visitor<'de> for NaiveDateTimeVisitor {
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatementItem {
    id: Option<String>,
    action: Option<String>,
    amount: Option<i64>,
    status: Option<StatementItemStatus>,
    #[serde(deserialize_with = "from_timestamp")]
    created_at: NaiveDateTime,
    data: Option<StatementItemData>,
    normalized_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum StatementItemStatus {
    #[serde(rename = "CONFIRMED")]
    Confirmed,
    #[serde(rename = "REFUNDED")]
    Refunded,
    #[serde(rename = "PENDING")]
    Pending,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatementItemData {
    merchant_name: Option<String>,
    operation_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatementQuery {
    limit: usize,
    cursor: Option<String>,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
}

impl StatementQuery {
    pub fn with_cursor(mut self, cursor: Option<String>) -> Self {
        self.cursor = cursor;
        self
    }

    pub fn with_date_range(mut self, range: Option<(NaiveDate, NaiveDate)>) -> Self {
        match range {
            Some((start_date, end_date)) => {
                self.start_date = Some(start_date);
                self.end_date = Some(end_date);
            }
            None => {
                self.start_date = None;
                self.end_date = None;
            }
        }
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
}

impl Default for StatementQuery {
    fn default() -> Self {
        Self {
            limit: 2,
            cursor: None,
            start_date: None,
            end_date: None,
        }
    }
}

pub struct CajuClient {
    base_url: String,
    user_id: String,
    employee_id: String,
    client: reqwest::Client,
}

impl CajuClient {
    pub fn new(base_url: String, user_id: String, employee_id: String) -> anyhow::Result<Self> {
        Ok(Self {
            base_url,
            user_id,
            employee_id,
            client: reqwest::Client::builder().build()?,
        })
    }

    pub async fn login(
        &mut self,
        existing_auth: &str,
        refresh_token: &str,
    ) -> anyhow::Result<LoginResponse> {
        let resp: LoginResponse = self
            .client
            .post(format!("{}/v1/user/{}/bearer_token", self.base_url, self.user_id).as_str())
            .header("Authorization", format!("Bearer {}", existing_auth))
            .body(
                json!({
                    "refreshToken": refresh_token,
                })
                .to_string(),
            )
            .send()
            .await?
            .json()
            .await?;

        let mut default_headers = HeaderMap::new();
        default_headers.append(
            "Authorization",
            format!("Bearer {}", resp.bearer_token).parse()?,
        );

        self.client = reqwest::Client::builder()
            .default_headers(default_headers)
            .build()?;

        Ok(resp)
    }

    pub async fn get_statement(&self, query: StatementQuery) -> anyhow::Result<StatementResponse> {
        let response = self
            .client
            .get(
                format!(
                    "{}/v1/employee/{}/statement",
                    self.base_url, self.employee_id
                )
                .as_str(),
            )
            .query(&[
                ("limit", query.limit.to_string()),
                ("cursor", query.cursor.unwrap_or_default()),
                ("order", "DESC".to_string()),
                (
                    "start_date",
                    query
                        .start_date
                        .map(|d| d.format("%F").to_string())
                        .unwrap_or_default(),
                ),
                (
                    "end_date",
                    query
                        .end_date
                        .map(|d| d.format("%F").to_string())
                        .unwrap_or_default(),
                ),
            ])
            .send()
            .await?
            .text()
            .await?;

        serde_json::from_str::<StatementResponse>(&response).map_err(|e| {
            anyhow::anyhow!(format!(
                "Failed to parse response: {}.\nResponse: {}",
                e, response
            ))
        })
    }

    pub async fn get_month_statement(
        &self,
        year: Option<i32>,
        month: chrono::Month,
    ) -> anyhow::Result<Vec<StatementItem>> {
        let first_day_of_month = NaiveDate::from_ymd_opt(
            year.unwrap_or_else(|| chrono::Local::now().year()),
            month.number_from_month(),
            1,
        )
        .ok_or(anyhow::anyhow!("Failed to get current month"))?;

        let last_day_of_month = first_day_of_month
            .checked_add_months(Months::new(1))
            .ok_or(anyhow::anyhow!("Failed to add a month to current month"))?
            .pred_opt()
            .ok_or(anyhow::anyhow!("Failed to get last day"))?;

        let mut has_next = true;
        let mut cursor = None;
        let mut statements = vec![];
        while has_next {
            let resp = self
                .get_statement(
                    StatementQuery::default()
                        .with_date_range(Some((first_day_of_month, last_day_of_month)))
                        .with_cursor(cursor)
                        .with_limit(20),
                )
                .await?;

            has_next = resp.has_next;
            if let Some(first) = resp.items.last() {
                cursor = first.cursor.clone();
            } else {
                break;
            }

            let mut items: Vec<_> = resp.items.into_iter().map(|i| i.item).collect();
            statements.append(&mut items);
        }

        Ok(statements)
    }
}

impl TryFrom<Vec<StatementItem>> for Ofx {
    type Error = anyhow::Error;

    fn try_from(value: Vec<StatementItem>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(anyhow::anyhow!("No statement to convert"));
        }
        let start = value.first().unwrap().created_at;
        let end = value.last().unwrap().created_at;
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
                            bank_id: "Caju".to_string(),
                        },
                        transactions: OfxTransactions {
                            start: start.format("%Y%m%d000000[-3:BRT]").to_string(),
                            end: end.format("%Y%m%d000000[-3:BRT]").to_string(),
                            transactions: value
                                .into_iter()
                                .filter(|statement| {
                                    statement.status == Some(StatementItemStatus::Confirmed)
                                })
                                .map(|statement| {
                                    OfxTransactionVariant::Transaction(crate::ofx::OfxTransaction {
                                        description: statement
                                            .data
                                            .and_then(|d| d.merchant_name)
                                            .unwrap_or_else(|| {
                                                if let Some(action) = statement.action.as_ref() {
                                                    if action == "CREDIT" {
                                                        return "Depósito em conta".to_string();
                                                    }
                                                }
                                                "unknown".to_string()
                                            }),
                                        type_: statement
                                            .action
                                            .clone()
                                            .unwrap_or("DEBIT".to_string()),
                                        timestamp: statement
                                            .created_at
                                            .format("%Y%m%d000000[-3:BRT]")
                                            .to_string(),
                                        amount: format!(
                                            "{:.2}",
                                            (statement.amount.unwrap_or(0) as f64) / 100.0
                                                * if statement.action.unwrap_or("DEBIT".to_string())
                                                    == "DEBIT"
                                                {
                                                    -1.0
                                                } else {
                                                    1.0
                                                }
                                        ),
                                        id: statement.id.unwrap_or_default(),
                                    })
                                })
                                .collect(),
                        },
                    },
                },
            }),
        })
    }
}
