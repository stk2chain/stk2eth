# USSD Client

HTTP→WebSocket bridge USSD client for handling Africa's Talking gateway requests and communicating with SpacetimeDB.

## Run

```bash
source .env

python ussdclient_v2.py [--debug]
```

## USSD Session
```json
{
    "sessionId"       : "ATUid_ae6b810be7d61d6bd67ad124c493a5c3",
    "phoneNumber"   : "+254712345678",
    "networkCode"   : "99999",
    "serviceCode"     : "*4337#",
    "text"             : "1*+10000...*2,622.36*xxxxxx"
}
```
## API Endpoint

###  POST `/ussdeth`
**In:** `sessionId`, `phoneNumber`, `networkCode`, `serviceCode`, `text`  
**Out:** `"CON <text>"` or `"END <text>"`



### Flow
```
POST /ussdeth → call_reducer("process_ussd_step", ...) → query("ussd_response") → return text
```

**Example:**
```python
@app.route("/ussdeth", methods=['POST'])
def ussd():
    ...
    try:
        stdb.call_reducer("process_ussd_step", session_id, phone, network, service, text)

        sql = f"SELECT * FROM ussd_response WHERE session_id = '{session_id}'"
        result = stdb.query(sql)
        success, rows = stdb.parse_query(result)

        response_text = rows[-1].get("response_text", "END No response")
        return response_text
```
```python
# Request
{"sessionId": "AT123", "phoneNumber": "+254712345678", "text": "1*0.5*1234"}

# Execution
stdb.call_reducer("process_ussd_step", "AT123", "+254712345678", "63902", "*384#", "1*0.5*1234")
stdb.query("SELECT * FROM ussd_response WHERE session_id = 'AT123'")

# Response
"END Sent 0.5 ETH"  # or "CON Enter PIN:"
```

## Session Lifecycle

```
Dial *384#       → text=""              → "CON 1.Send ETH\n2.Balance"
Press 1          → text="1"             → "CON Enter phone:"
Enter +254700... → text="1*+254700..."  → "CON Enter amount:"
Enter 0.5        → text="1*+254700*0.5" → "CON Enter PIN:"
Enter 1234       → text="1*+254700*0.5*1234" → "END Sending..."
```


## SpacetimeDB Contract

```rust
// Reducer
process_ussd_step(session_id: String, phone: String, network: String, service: String, text: String)

// Table
ussd_response { session_id: String, response_text: String }
```

## Errors

| Condition | Response |
|-----------|----------|
| Timeout (>10s) | `END Request timeout` |
| Disconnected | `END Service unavailable` |
| Empty result | `END No response` |
| Query failed | `END Database error` |

## Config

```env
SPACETIMEDB_HOST=0.0.0.0
SPACETIMEDB_DBNAME=gateway2
SPACETIMEDB_PORT=3000
```

