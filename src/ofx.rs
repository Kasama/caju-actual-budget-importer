use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "OFX")]
pub struct Ofx {
    #[serde(rename = "BANKMSGSRSV1")]
    pub bank: Option<OfxBanking>,
    #[serde(rename = "CREDITCARDMSGSRSV1")]
    pub credit_card: Option<OfxCreditCard>,
}

impl Ofx {
    pub fn to_ofx(&self) -> Result<String, serde_xml_rs::Error> {
        serde_xml_rs::to_string(&self)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "BANKMSGSRSV1")]
pub struct OfxBanking {
    #[serde(rename = "STMTTRNRS")]
    pub statement: OfxBankingStatement,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OfxBankingStatement {
    #[serde(rename = "TRNUID")]
    pub transaction_id: String,
    #[serde(rename = "STATUS")]
    pub status: OfxStatementStatus,
    #[serde(rename = "STMTRS")]
    pub statements: OfxStatement,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "CREDITCARDMSGSRSV1")]
pub struct OfxCreditCard {
    #[serde(rename = "CCSTMTTRNRS")]
    pub statement: OfxCreditCardStatement,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OfxCreditCardStatement {
    #[serde(rename = "TRNUID")]
    pub transaction_id: String,
    #[serde(rename = "STATUS")]
    pub status: OfxStatementStatus,
    #[serde(rename = "CCSTMTRS")]
    pub statements: OfxStatement,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OfxStatementStatus {
    #[serde(rename = "CODE")]
    pub code: usize,
    #[serde(rename = "SEVERITY")]
    pub severity: String,
}

///
/// <STMTRS>
///   <CURDEF>BRL</CURDEF>
///   <BANKACCTFROM> ... </BANKACCTFROM>
///   <BANKTRANLIST> ... </BANKTRANLIST>
///   <LEDGERBAL> ... </LEDGERBAL>
///   <BALLIST> ... </BALLIST>
/// </STMTRS>
#[derive(Debug, Serialize, Deserialize)]
pub struct OfxStatement {
    #[serde(rename = "CURDEF")]
    pub currency_code: String,
    #[serde(rename = "BANKACCTFROM")]
    pub bank_account: OfxBankAccount,
    #[serde(rename = "BANKTRANLIST")]
    pub transactions: OfxTransactions,
    // #[serde(rename = "LEDGERBAL")]
    // pub ledger_balance: OfxLedgerBalance,
}

/// <BANKACCTFROM>
///   <BANKID>0000</BANKID>
///   <BRANCHID>0</BRANCHID>
///   <ACCTID>0000000-0</ACCTID>
///   <ACCTTYPE>CHECKING</ACCTTYPE>
/// </BANKACCTFROM>
#[derive(Debug, Serialize, Deserialize)]
pub struct OfxBankAccount {
    #[serde(rename = "BANKID")]
    pub bank_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "BANKTRANLIST")]
pub struct OfxTransactions {
    #[serde(rename = "DTSTART")]
    pub start: String,
    #[serde(rename = "DTEND")]
    pub end: String,
    #[serde(rename = "$value")]
    pub transactions: Vec<OfxTransactionVariant>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum OfxTransactionVariant {
    #[serde(rename = "STMTTRN")]
    Transaction(OfxTransaction),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "STMTTRN")]
pub struct OfxTransaction {
    #[serde(rename = "TRNTYPE")]
    pub type_: String,
    #[serde(rename = "DTPOSTED")]
    pub timestamp: String,
    #[serde(rename = "TRNAMT")]
    pub amount: String,
    #[serde(rename = "FITID")]
    pub id: String,
    #[serde(rename = "MEMO")]
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OfxLedgerBalance {}
