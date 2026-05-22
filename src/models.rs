use serde::{Deserialize, Serialize};

// -- Requests --

#[derive(Deserialize)]
pub struct PayRequest {
    pub invoice: String,
    pub amount: Option<u64>,
}

#[derive(Deserialize)]
pub struct CreateInvoiceRequest {
    pub amount_sats: u64,
    pub description: Option<String>,
    pub description_hash: Option<String>,
    pub expiry: Option<u64>,
}

#[derive(Deserialize)]
pub struct TransactionQuery {
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    #[serde(rename = "type")]
    pub transaction_type: Option<String>,
    pub unpaid: Option<bool>,
}

// -- Responses --

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Serialize)]
pub struct BalanceResponse {
    pub balance_msats: u64,
    pub balance_sats: u64,
}

#[derive(Serialize)]
pub struct PayResponse {
    pub preimage: String,
    pub fees_paid: Option<u64>,
}

#[derive(Serialize)]
pub struct CreateInvoiceResponse {
    pub invoice: String,
    pub payment_hash: Option<String>,
    pub expires_at: Option<u64>,
}

#[derive(Serialize)]
pub struct InfoResponse {
    pub alias: Option<String>,
    pub color: Option<String>,
    pub pubkey: Option<String>,
    pub network: Option<String>,
    pub block_height: Option<u32>,
    pub block_hash: Option<String>,
    pub methods: Vec<String>,
    pub notifications: Vec<String>,
}

#[derive(Serialize)]
pub struct InvoiceResponse {
    #[serde(rename = "type")]
    pub transaction_type: Option<String>,
    pub state: Option<String>,
    pub invoice: Option<String>,
    pub description: Option<String>,
    pub preimage: Option<String>,
    pub payment_hash: String,
    pub amount: u64,
    pub fees_paid: u64,
    pub created_at: Option<u64>,
    pub expires_at: Option<u64>,
    pub settled_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Serialize)]
pub struct TransactionsResponse {
    pub transactions: Vec<InvoiceResponse>,
}

// -- SSE Notifications --

#[derive(Serialize, Clone, Debug)]
pub struct EventNotification {
    pub notification_type: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub transaction_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preimage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fees_paid: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settled_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settle_deadline: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}
