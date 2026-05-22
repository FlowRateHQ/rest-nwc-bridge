# rest-nwc-bridge

HTTP-to-NWC gateway — a REST API server that translates HTTP requests to [Nostr Wallet Connect (NIP-47)](https://github.com/nostr-protocol/nips/blob/master/47.md) calls. This allows any HTTP client to interact with a Lightning wallet without implementing the Nostr protocol, WebSocket connections, or NIP-44 encryption.

## Setup

Requires Rust and the [nostr-rs-nwc](https://github.com/rust-nostr/nostr) crates checked out at `../nostr-rs-nwc`.

```sh
cargo build
```

## Usage

```sh
NWC_URI="nostr+walletconnect://..." cargo run
```

Environment variables:

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `NWC_URI` | yes | — | Nostr Wallet Connect URI |
| `PORT` | no | `8080` | HTTP listen port |
| `WEBHOOK_URL` | no | — | URL to POST event notifications to (disabled if unset) |
| `RUST_LOG` | no | `rest_nwc_bridge=info,nwc=info` | Log level filter |

## API

All endpoints return JSON. Errors return `{ "error": "message" }` with an appropriate HTTP status code.

### GET /info

Returns wallet info.

```sh
curl http://localhost:8080/info
```

```json
{
  "alias": "my-node",
  "color": "#ff9900",
  "pubkey": "03...",
  "network": "mainnet",
  "block_height": 850000,
  "block_hash": "0000...",
  "methods": ["pay_invoice", "get_balance", "make_invoice", "lookup_invoice", "list_transactions"],
  "notifications": []
}
```

### GET /balance

Returns wallet balance.

```sh
curl http://localhost:8080/balance
```

```json
{ "balance_msats": 87500000, "balance_sats": 87500 }
```

### POST /pay

Pays a bolt11 invoice.

```sh
curl -X POST http://localhost:8080/pay \
  -H 'Content-Type: application/json' \
  -d '{"invoice": "lnbc1..."}'
```

Request body:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `invoice` | string | yes | BOLT-11 invoice |
| `amount` | integer | no | Amount in msats (for zero-amount invoices) |

```json
{ "preimage": "abc123...", "fees_paid": 1000 }
```

### POST /invoice

Creates a new invoice.

```sh
curl -X POST http://localhost:8080/invoice \
  -H 'Content-Type: application/json' \
  -d '{"amount_sats": 100, "description": "test"}'
```

Request body:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `amount_sats` | integer | yes | Amount in sats |
| `description` | string | no | Invoice description |
| `description_hash` | string | no | SHA256 hash of description |
| `expiry` | integer | no | Expiry in seconds |

```json
{ "invoice": "lnbc1...", "payment_hash": "0b86...", "expires_at": 1716300000 }
```

### GET /invoice/:payment_hash

Looks up an invoice by payment hash.

```sh
curl http://localhost:8080/invoice/0b86af...
```

```json
{
  "type": "incoming",
  "state": "settled",
  "invoice": "lnbc1...",
  "description": "test",
  "preimage": "abc...",
  "payment_hash": "0b86af...",
  "amount": 100000,
  "fees_paid": 0,
  "created_at": 1716290000,
  "expires_at": 1716300000,
  "settled_at": 1716291000
}
```

### GET /transactions

Lists transactions with optional filters.

```sh
curl "http://localhost:8080/transactions?limit=10&offset=0&type=incoming"
```

Query parameters:

| Param | Type | Description |
|-------|------|-------------|
| `limit` | integer | Max number of results |
| `offset` | integer | Pagination offset |
| `type` | string | `incoming` or `outgoing` |
| `unpaid` | boolean | Include unpaid transactions |

```json
{
  "transactions": [
    {
      "type": "incoming",
      "state": "settled",
      "payment_hash": "...",
      "amount": 100000,
      "fees_paid": 0,
      "settled_at": 1716291000
    }
  ]
}
```

### GET /events

Streams real-time wallet notifications via [Server-Sent Events (SSE)](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events). The connection stays open and pushes events as they occur.

```sh
curl -N http://localhost:8080/events
```

Event types: `payment_received`, `payment_sent`, `hold_invoice_accepted`.

Example output:

```
event: payment_received
data: {"notification_type":"payment_received","type":"incoming","invoice":"lntbs...","preimage":"abc...","payment_hash":"0b86...","amount":100000,"fees_paid":0,"settled_at":1716291000}

event: payment_sent
data: {"notification_type":"payment_sent","type":"outgoing","invoice":"lntbs...","preimage":"def...","payment_hash":"7c3f...","amount":50000,"fees_paid":100,"settled_at":1716291500}

event: hold_invoice_accepted
data: {"notification_type":"hold_invoice_accepted","type":"incoming","invoice":"lntbs...","payment_hash":"9a1b...","amount":200000,"created_at":1716290000,"expires_at":1716300000,"settle_deadline":850100}
```

From JavaScript:

```js
const events = new EventSource("http://localhost:8080/events");
events.addEventListener("payment_received", (e) => {
  console.log(JSON.parse(e.data));
});
```

### Webhooks

As an alternative to SSE, you can configure `WEBHOOK_URL` to receive notifications via HTTP POST. When set, the bridge will POST each event as JSON to the configured URL:

```sh
WEBHOOK_URL=http://localhost:3000/webhook cargo run
```

Each notification is delivered as a POST request with `Content-Type: application/json`. The body is the same JSON object as SSE `data:` lines:

```
POST /webhook HTTP/1.1
Content-Type: application/json

{"notification_type":"payment_received","type":"incoming","invoice":"lntbs...","payment_hash":"0b86...","amount":100000,"fees_paid":0,"settled_at":1716291000}
```

If the webhook endpoint is unreachable, the bridge logs a warning and continues processing subsequent notifications.
