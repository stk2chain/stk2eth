"""
Minimal SpacetimeDB Client - Functional Programming Approach

A lightweight, functional client for SpacetimeDB with only essential features.
No classes, no OOP, just pure functions with comprehensive logging.
"""
import asyncio
import json
import uuid
import logging
from typing import Dict, Any, Optional, Callable
import websockets


# Configure logging
logger = logging.getLogger(__name__)


# ============================================================================
# CONNECTION MANAGEMENT
# ============================================================================

async def connect(uri: str, handlers: Dict[str, Callable] = None) -> tuple:
    """
    Connect to SpacetimeDB and return (websocket, tasks)
    
    Args:
        uri: WebSocket URI (e.g., "ws://localhost:3000/v1/database/mydb/subscribe")
        handlers: Optional dict of message_type -> handler_function
    
    Returns:
        (websocket, [tasks]) tuple
    """
    logger.info(f"Connecting to SpacetimeDB: {uri}")
    
    try:
        ws = await websockets.connect(
            uri,
            subprotocols=["v1.json.spacetimedb"],
            max_size=10 * 1024 * 1024
        )
        logger.info(f"✓ Connected successfully (subprotocol: {ws.subprotocol})")
    except Exception as e:
        logger.error(f"✗ Connection failed: {e}")
        raise
    
    pending = {}
    tasks = [
        asyncio.create_task(_handle_messages(ws, pending, handlers or {})),
        asyncio.create_task(_keep_alive(ws))
    ]
    
    # Store pending requests in websocket for access
    ws._pending = pending
    logger.debug(f"Background tasks started: message_handler, keep_alive")
    
    return ws, tasks


async def disconnect(ws, tasks: list):
    """Disconnect and cleanup"""
    logger.info("Disconnecting from SpacetimeDB")
    
    for task in tasks:
        task.cancel()
        logger.debug(f"Cancelled task: {task.get_name()}")
    
    await asyncio.gather(*tasks, return_exceptions=True)
    await ws.close()
    
    logger.info("✓ Disconnected successfully")


# ============================================================================
# MESSAGE HANDLING
# ============================================================================

async def _handle_messages(ws, pending: dict, handlers: dict):
    """Handle incoming messages"""
    msg_count = 0
    logger.debug("Message handler started")
    
    try:
        async for msg in ws:
            msg_count += 1
            logger.debug(f"← Message #{msg_count} received ({len(msg)} bytes)")
            
            try:
                data = json.loads(msg)
                msg_type = _get_message_type(data)
                logger.debug(f"  Type: {msg_type}")
                
                # Route to handlers
                for handler_type, handler in handlers.items():
                    if handler_type in data:
                        logger.debug(f"  Routing to handler: {handler_type}")
                        await _safe_call(handler, data)
                
                # Resolve pending requests
                resolved = _resolve_pending(data, pending)
                if resolved:
                    logger.debug(f"  ✓ Resolved pending request: {resolved}")
                    
            except json.JSONDecodeError as e:
                logger.warning(f"  ✗ Failed to parse JSON: {e}")
            except Exception as e:
                logger.error(f"  ✗ Error handling message: {e}")
                
    except websockets.exceptions.ConnectionClosed as e:
        logger.warning(f"WebSocket closed: code={e.code}, reason={e.reason}")
    except asyncio.CancelledError:
        logger.debug("Message handler cancelled")
    except Exception as e:
        logger.error(f"Message handler error: {e}", exc_info=True)
    
    logger.debug(f"Message handler stopped (processed {msg_count} messages)")


def _get_message_type(data: dict) -> str:
    """Extract message type from data"""
    for key in ['IdentityToken', 'TransactionUpdate', 'OneOffQueryResponse', 'SubscriptionUpdate']:
        if key in data:
            return key
    return 'Unknown'


def _resolve_pending(data: dict, pending: dict) -> Optional[str]:
    """Resolve pending request futures. Returns request identifier if resolved."""
    # TransactionUpdate with request_id
    if 'TransactionUpdate' in data:
        tx = data['TransactionUpdate']
        req_id = tx.get('request_id')
        status = tx.get('status', {})
        
        # If request_id is present and in pending
        if req_id is not None and req_id in pending:
            if 'Committed' in status:
                logger.debug(f"  Transaction committed: request_id={req_id}")
                pending[req_id].set_result(data)
                return f"request_id={req_id}"
            elif 'Failed' in status:
                error = status['Failed']
                logger.warning(f"  Transaction failed: request_id={req_id}, error={error}")
                pending[req_id].set_exception(Exception(error))
                return f"request_id={req_id}"
        
        # If no request_id or not in pending, resolve the oldest pending request
        # This handles cases where SpacetimeDB doesn't return request_id
        elif pending:
            # Get the oldest (lowest) request_id
            oldest_id = min(k for k in pending.keys() if isinstance(k, int))
            if 'Committed' in status:
                logger.debug(f"  Transaction committed (no request_id), resolving oldest: {oldest_id}")
                pending[oldest_id].set_result(data)
                return f"oldest_request={oldest_id}"
            elif 'Failed' in status:
                error = status['Failed']
                logger.warning(f"  Transaction failed (no request_id), resolving oldest: {oldest_id}, error={error}")
                pending[oldest_id].set_exception(Exception(error))
                return f"oldest_request={oldest_id}"
    
    # OneOffQueryResponse with message_id
    elif 'OneOffQueryResponse' in data:
        msg_id = data['OneOffQueryResponse'].get('message_id')
        if msg_id and msg_id in pending:
            logger.debug(f"  Query response received: message_id={msg_id}")
            pending[msg_id].set_result(data)
            return f"message_id={msg_id}"
    
    return None


async def _safe_call(fn: Callable, *args):
    """Safely call handler function"""
    try:
        if asyncio.iscoroutinefunction(fn):
            await fn(*args)
        else:
            fn(*args)
    except Exception as e:
        logger.error(f"Handler error: {e}", exc_info=True)


async def _keep_alive(ws):
    """Send periodic pings"""
    ping_count = 0
    logger.debug("Keep-alive task started")
    
    try:
        while True:
            await asyncio.sleep(20)
            ping_count += 1
            
            try:
                await ws.ping()
                logger.debug(f"↔ Ping #{ping_count} sent")
            except Exception as e:
                logger.error(f"✗ Ping failed: {e}")
                break
    except asyncio.CancelledError:
        logger.debug("Keep-alive task cancelled")
    
    logger.debug(f"Keep-alive task stopped (sent {ping_count} pings)")


# ============================================================================
# REDUCER CALLS
# ============================================================================

_request_counter = 0

async def call_reducer(ws, name: str, *args, timeout: float = 10) -> dict:
    """
    Call a SpacetimeDB reducer via websocket
    
    Args:
        ws: WebSocket connection
        name: Reducer name
        *args: Reducer arguments
        timeout: Request timeout
    
    Returns:
        Response data
    """
    global _request_counter
    _request_counter += 1
    req_id = _request_counter
    
    logger.info(f"→ Calling reducer: {name}")
    logger.debug(f"  request_id: {req_id}")
    # logger.debug(f"  args: {args}")
    logger.debug(f"  timeout: {timeout}s")
    
    future = asyncio.get_event_loop().create_future()
    ws._pending[req_id] = future
    
    msg = {
        "CallReducer": {
            "reducer": name,
            "args": json.dumps(list(args)),
            "request_id": req_id,
            "flags": 0
        }
    }
    
    try:
        await ws.send(json.dumps(msg))
        logger.debug(f"  ✓ Request sent")
        
        result = await asyncio.wait_for(future, timeout)
        logger.info(f"← Reducer response received: {name}")
        return result
        
    except asyncio.TimeoutError:
        logger.error(f"✗ Reducer timeout: {name} (after {timeout}s)")
        raise
    except Exception as e:
        logger.error(f"✗ Reducer error: {name} - {e}")
        raise
    finally:
        ws._pending.pop(req_id, None)
        logger.debug(f"  Cleaned up request_id: {req_id}")


# ============================================================================
# QUERIES
# ============================================================================

async def query(ws, sql: str, timeout: float = 10) -> dict:
    """
    Execute SQL query
    
    Args:
        ws: WebSocket connection
        sql: SQL query string
        timeout: Request timeout
    
    Returns:
        Query response data
    """
    msg_id = uuid.uuid4().hex
    
    logger.info(f"→ Executing query")
    logger.debug(f"  message_id: {msg_id}")
    # logger.debug(f"  sql: {sql[:100]}{'...' if len(sql) > 100 else ''}")
    logger.debug(f"  timeout: {timeout}s")
    
    future = asyncio.get_event_loop().create_future()
    ws._pending[msg_id] = future
    
    msg = {
        "OneOffQuery": {
            "message_id": msg_id,
            "query_string": sql
        }
    }
    
    try:
        await ws.send(json.dumps(msg))
        logger.debug(f"  ✓ Query sent")
        
        result = await asyncio.wait_for(future, timeout)
        logger.info(f"← Query response received")
        return result
        
    except asyncio.TimeoutError:
        logger.error(f"✗ Query timeout (after {timeout}s)")
        raise
    except Exception as e:
        logger.error(f"✗ Query error: {e}")
        raise
    finally:
        ws._pending.pop(msg_id, None)
        logger.debug(f"  Cleaned up message_id: {msg_id}")


# ============================================================================
# SUBSCRIPTIONS
# ============================================================================  
async def send_subscription(ws, table_name: str, timeout: float = 10):
    """Send SubscribeSingle message to subscribe to Swap table"""
    global _request_counter
    _request_counter += 1
    req_id = _request_counter
    
    logger.info(f"→ Subscription to table: {table_name}")
    logger.debug(f"  request_id: {req_id}")
    # logger.debug(f"  args: {args}")
    logger.debug(f"  timeout: {timeout}s")
    

    try:
        # Generate unique IDs
        query_id = int(uuid.uuid4().int & 0xFFFFFFFF)

        # Construct SubscribeSingle message
        msg = {
            "SubscribeSingle": {
                "query": f"SELECT * FROM {table_name}",
                "request_id": req_id,
                "query_id": {"id": query_id}
            }
        }

        json_message = json.dumps(msg)

        logger.info(f"→ Sending subscription request")
        logger.info(f"  Query: SELECT * FROM {table_name}")
        logger.info(f"  Request ID: {req_id}")
        logger.info(f"  Query ID: {query_id}")

        await ws.send(json_message)
        logger.info(f"✓ Subscription request sent\n")

    except Exception as e:
        logger.error(f"✗ Failed to send subscription: {e}", exc_info=True)


# ============================================================================
# RESULT PARSING
# ============================================================================

def parse_query(response: dict) -> tuple[bool, Any]:
    """
    Parse query response
    
    Returns:
        (success, data_or_error)
    """
    logger.debug("Parsing query response")
    
    qr = response.get('OneOffQueryResponse', {})
    error = qr.get('error', {})
    
    if 'some' in error:
        error_msg = error['some']
        logger.warning(f"  Query returned error: {error_msg}")
        return False, error_msg
    
    tables = qr.get('tables', [])
    if not tables:
        logger.debug("  No tables in response")
        return True, []
    
    rows = tables[0].get('rows', [])
    parsed_rows = [json.loads(r) if isinstance(r, str) else r for r in rows]
    
    logger.debug(f"  ✓ Parsed {len(parsed_rows)} rows from {len(tables)} table(s)")
    return True, parsed_rows


def parse_reducer(response: dict) -> tuple[bool, Any]:
    """
    Parse reducer response
    
    Returns:
        (success, data_or_error)
    """
    logger.debug("Parsing reducer response")
    
    tx = response.get('TransactionUpdate', {})
    status = tx.get('status', {})
    
    if 'Failed' in status:
        error = status['Failed']
        logger.warning(f"  Reducer returned error: {error}")
        return False, error
    if 'Committed' in status:
        logger.debug(f"  ✓ Reducer committed successfully")
        return True, tx
    
    logger.warning(f"  Unknown reducer status: {status}")
    return False, "Unknown status"


def filter_rows(rows: list, **filters) -> list:
    """Filter rows by field values"""
    logger.debug(f"Filtering {len(rows)} rows by: {filters}")
    
    result = [r for r in rows if all(r.get(k) == v for k, v in filters.items())]
    
    logger.debug(f"  ✓ Filtered to {len(result)} rows")
    return result


def find_one(rows: list, **filters) -> Optional[dict]:
    """Find first matching row"""
    logger.debug(f"Finding one row from {len(rows)} rows by: {filters}")
    
    matches = filter_rows(rows, **filters)
    result = matches[0] if matches else None
    
    if result:
        logger.debug(f"  ✓ Found matching row")
    else:
        logger.debug(f"  ✗ No matching row found")
    
    return result


# ============================================================================
# FLASK INTEGRATION
# ============================================================================

class FlaskSTDB:
    """Minimal Flask integration - single class for lifecycle management"""
    
    def __init__(self, uri: str, handlers: dict = None):
        self.uri = uri
        self.handlers = handlers or {}
        self.ws = None
        self.tasks = []
        self.loop = None
        self.ready = asyncio.Event()
        
        logger.info(f"FlaskSTDB initialized: {uri}")
    
    def start(self):
        """Start background client"""
        import threading
        
        logger.info("Starting background SpacetimeDB client thread")
        
        def run():
            self.loop = asyncio.new_event_loop()
            asyncio.set_event_loop(self.loop)
            logger.debug("Event loop created in background thread")
            
            try:
                self.loop.run_until_complete(self._connect())
            except Exception as e:
                logger.error(f"Background thread error: {e}", exc_info=True)
        
        thread = threading.Thread(target=run, daemon=True, name="SpacetimeDB-Client")
        thread.start()
        logger.debug(f"Background thread started: {thread.name}")
    
    async def _connect(self):
        """Connect and maintain"""
        logger.info("Background client connecting...")
        
        try:
            self.ws, self.tasks = await connect(self.uri, self.handlers)
            self.ready.set()
            logger.info("✓ Background client ready")
            
            # Keep running
            await asyncio.gather(*self.tasks, return_exceptions=True)
        except Exception as e:
            logger.error(f"Background client error: {e}", exc_info=True)
    
    def call_reducer(self, name: str, *args, timeout: float = 10) -> dict:
        """Call reducer (blocking)"""
        logger.debug(f"FlaskSTDB.call_reducer: {name} (blocking call from Flask)")
        
        if not self.ready.is_set():
            logger.error("✗ Client not ready")
            raise ConnectionError("SpacetimeDB client not ready")
        
        future = asyncio.run_coroutine_threadsafe(
            call_reducer(self.ws, name, *args, timeout=timeout),
            self.loop
        )
        
        try:
            result = future.result(timeout)
            logger.debug(f"✓ Blocking call completed: {name}")
            return result
        except Exception as e:
            logger.error(f"✗ Blocking call failed: {name} - {e}")
            raise
    
    def query(self, sql: str, timeout: float = 10) -> dict:
        """Execute query (blocking)"""
        logger.debug(f"FlaskSTDB.query (blocking call from Flask)")
        
        if not self.ready.is_set():
            logger.error("✗ Client not ready")
            raise ConnectionError("SpacetimeDB client not ready")
        
        future = asyncio.run_coroutine_threadsafe(
            query(self.ws, sql, timeout=timeout),
            self.loop
        )
        
        try:
            result = future.result(timeout)
            logger.debug(f"✓ Blocking query completed")
            return result
        except Exception as e:
            logger.error(f"✗ Blocking query failed: {e}")
            raise
            

    def send_subscription(self, name: str, timeout: float = 10):
        """Send subscription (blocking)"""
        logger.debug(f"FlaskSTDB.send_subscription: {name} (blocking call from Flask)")
        
        if not self.ready.is_set():
            logger.error("✗ Client not ready")
            raise ConnectionError("SpacetimeDB client not ready")
        
        future = asyncio.run_coroutine_threadsafe(
            send_subscription(self.ws, name, timeout=timeout),
            self.loop
        )
        
        try:
            result = future.result()
            logger.debug(f"✓ Blocking subscription sent: {name}")
            return result
        except Exception as e:
            logger.error(f"✗ Blocking subscription failed: {name} - {e}")
            raise

# ============================================================================
# EXAMPLE USAGE
# ============================================================================

async def example_standalone():
    """Example: Standalone async usage"""
    uri = "ws://localhost:3000/v1/database/mydb/subscribe"
    
    # Optional: Define message handlers
    async def on_transaction(data):
        logger.info(f"Transaction handler called: {data}")
    
    handlers = {"TransactionUpdate": on_transaction}
    
    # Connect
    ws, tasks = await connect(uri, handlers)
    
    try:
        # Call reducer
        result = await call_reducer(ws, "process_data", "arg1", "arg2")
        success, data = parse_reducer(result)
        logger.info(f"Reducer result: success={success}, data={data}")
        
        # Execute query
        result = await query(ws, "SELECT * FROM users")
        success, rows = parse_query(result)
        if success:
            logger.info(f"Query returned {len(rows)} rows")
            
            # Filter results
            active = filter_rows(rows, status="active")
            logger.info(f"Active users: {len(active)}")
            
            # Find specific row
            user = find_one(rows, name="Alice")
            logger.info(f"Found user: {user}")
        
        # Keep running
        await asyncio.sleep(60)
        
    finally:
        await disconnect(ws, tasks)


def example_flask():
    """Example: Flask integration"""
    from flask import Flask, request
    
    app = Flask(__name__)
    
    # Initialize client
    uri = "ws://localhost:3000/v1/database/gateway2/subscribe"
    stdb = FlaskSTDB(uri)
    stdb.start()
    
    @app.route("/ussd", methods=['POST'])
    def ussd():
        session_id = request.values.get("sessionId")
        phone = request.values.get("phoneNumber")
        text = request.values.get("text", "")
        
        logger.info(f"USSD request: session={session_id}, phone={phone}")
        
        # Call reducer
        stdb.call_reducer("process_ussd_step", session_id, phone, text)
        
        # Query response
        sql = f"SELECT * FROM ussd_response WHERE session_id = '{session_id}'"
        result = stdb.query(sql)
        success, rows = parse_query(result)
        
        if success and rows:
            response = rows[-1].get("response_text", "END Error")
            logger.info(f"USSD response: {response}")
            return response
        
        logger.warning("No USSD response found")
        return "END No response"
    
    @app.route("/health")
    def health():
        is_ready = stdb.ready.is_set()
        logger.debug(f"Health check: ready={is_ready}")
        return {"status": "ok", "connected": is_ready}
    
    return app


# ============================================================================
# HELPER: BUILD URI
# ============================================================================

def build_uri(host: str = "localhost", port: int = 3000, 
              database: str = "gateway2", secure: bool = False) -> str:
    """Build SpacetimeDB WebSocket URI"""
    proto = "wss" if secure else "ws"
    uri = f"{proto}://{host}:{port}/v1/database/{database}/subscribe"
    logger.debug(f"Built URI: {uri}")
    return uri


# ============================================================================
# LOGGING CONFIGURATION
# ============================================================================

def configure_logging(level=logging.INFO, format_string=None):
    """
    Configure logging for the module
    
    Args:
        level: Logging level (DEBUG, INFO, WARNING, ERROR)
        format_string: Custom format string (optional)
    """
    if format_string is None:
        format_string = '%(asctime)s [%(levelname)s] %(name)s: %(message)s'
    
    logging.basicConfig(
        level=level,
        format=format_string,
        datefmt='%Y-%m-%d %H:%M:%S'
    )
    
    # Set module logger level
    logger.setLevel(level)
    
    logger.info(f"Logging configured: level={logging.getLevelName(level)}")


if __name__ == "__main__":
    # Configure logging
    configure_logging(level=logging.DEBUG)
    
    # Run standalone example
    asyncio.run(example_standalone())