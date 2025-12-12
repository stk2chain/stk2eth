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
        logger.debug("Attempting websocket connection...")
        ws = await websockets.connect(
            uri,
            subprotocols=["v1.json.spacetimedb"],
            max_size=10 * 1024 * 1024
        )
        logger.info(f"✓ WebSocket connected successfully (subprotocol: {ws.subprotocol})")
    except Exception as e:
        logger.error(f"✗ WebSocket connection failed: {e}")
        raise

    pending = {}

    logger.debug("Creating background tasks...")
    tasks = [
        asyncio.create_task(_handle_messages(ws, pending, handlers or {})),
        asyncio.create_task(_keep_alive(ws))
    ]
    logger.debug(f"✓ Created {len(tasks)} background tasks")

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
    Call a SpacetimeDB reducer

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
    logger.debug(f"  args: {args}")
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
    logger.debug(f"  sql: {sql[:100]}{'...' if len(sql) > 100 else ''}")
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
            try:
                self.loop = asyncio.new_event_loop()
                asyncio.set_event_loop(self.loop)
                logger.debug("Event loop created in background thread")

                self.loop.run_until_complete(self._connect())

            except KeyboardInterrupt:
                logger.info("Background thread interrupted")
            except Exception as e:
                logger.error(f"Background thread error: {e}", exc_info=True)
            finally:
                # Cleanup
                if self.loop and not self.loop.is_closed():
                    logger.debug("Closing event loop")
                    pending = asyncio.all_tasks(self.loop)
                    for task in pending:
                        task.cancel()
                    self.loop.run_until_complete(asyncio.gather(*pending, return_exceptions=True))
                    self.loop.close()

        thread = threading.Thread(target=run, daemon=True, name="SpacetimeDB-Client")
        thread.start()
        logger.debug(f"Background thread started: {thread.name}")

        # Wait briefly for connection to establish
        import time
        timeout = 10
        start_time = time.time()
        while not self.ready.is_set() and (time.time() - start_time) < timeout:
            time.sleep(0.1)

        if self.ready.is_set():
            logger.info("✓ Client ready for requests")
        else:
            logger.warning(f"Client not ready after {timeout}s - may still be connecting")

    async def _connect(self):
        """Connect and maintain"""
        logger.info("Background client connecting...")

        try:
            self.ws, self.tasks = await connect(self.uri, self.handlers)
            logger.info("✓ Connection established")

            # Set ready immediately after connection
            self.ready.set()
            logger.info("✓ Background client marked as ready")

            # Keep running - wait for any task to fail
            done, pending = await asyncio.wait(
                self.tasks,
                return_when=asyncio.FIRST_COMPLETED
            )

            logger.warning("Background client task completed unexpectedly")

            # Cancel remaining tasks
            for task in pending:
                task.cancel()

        except Exception as e:
            logger.error(f"Background client error: {e}", exc_info=True)
            self.ready.clear()

    def call_reducer(self, name: str, *args, timeout: float = 10) -> dict:
        """Call reducer (blocking)"""
        logger.debug(f"FlaskSTDB.call_reducer: {name} (blocking call from Flask)")

        if not self.ready.is_set():
            logger.error(f"✗ Client not ready (ready={self.ready.is_set()}, ws={self.ws is not None}, loop={self.loop is not None})")
            raise ConnectionError("SpacetimeDB client not ready")

        if not self.ws:
            logger.error("✗ WebSocket is None")
            raise ConnectionError("WebSocket connection is None")

        if not self.loop:
            logger.error("✗ Event loop is None")
            raise ConnectionError("Event loop is None")

        logger.debug(f"Client state: ready={self.ready.is_set()}, ws={self.ws}, loop={self.loop}")

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
            logger.error(f"✗ Client not ready - ready={self.ready.is_set()}, ws={self.ws is not None}, loop={self.loop is not None}")
            raise ConnectionError("SpacetimeDB client not ready")

        if not self.ws:
            logger.error("✗ WebSocket connection is None")
            raise ConnectionError("WebSocket connection is None")

        if not self.loop or self.loop.is_closed():
            logger.error("✗ Event loop is None or closed")
            raise ConnectionError("Event loop is not available")

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


# ============================================================================
# SWAP TABLE SUBSCRIPTION HANDLER
# ============================================================================

def parse_swap_insert(tx_data: dict) -> Optional[dict]:
    """
    Parse swap table insert from TransactionUpdate

    Args:
        tx_data: TransactionUpdate data

    Returns:
        Parsed swap data or None if no swap insert found
    """
    try:
        tx = tx_data.get('TransactionUpdate', {})
        status = tx.get('status', {})

        if 'Committed' not in status:
            return None

        tables = status['Committed'].get('tables', [])

        # Find swap table
        for table in tables:
            if table.get('table_name') == 'swap':
                updates = table.get('updates', [])

                for update in updates:
                    inserts = update.get('inserts', [])

                    if inserts:
                        # Parse first insert (JSON array string)
                        insert_str = inserts[0]
                        insert_data = json.loads(insert_str)

                        logger.debug(f"Parsed swap insert: {len(insert_data)} fields")

                        # Map to swap structure (adjust indices based on your schema)
                        swap = {
                            'id': insert_data[0],
                            'session_id': insert_data[1],
                            'from_address': insert_data[2],
                            'to_address': insert_data[3],
                            'amount': insert_data[4],
                            'from_token': insert_data[5],
                            'to_token': insert_data[6],
                            # Optional fields with tuple unpacking
                            'gas_price': insert_data[10][1] if len(insert_data[10]) > 1 else None,
                            'created_at': insert_data[12][0] if len(insert_data) > 12 else None,
                            'updated_at': insert_data[13][0] if len(insert_data) > 13 else None,
                        }

                        logger.info(f"✓ Swap insert detected: session={swap['session_id']}, amount={swap['amount']}")
                        return swap

        return None

    except Exception as e:
        logger.error(f"Error parsing swap insert: {e}", exc_info=True)
        return None


async def handle_swap_transaction(data: dict, callback: Optional[Callable] = None):
    """
    Handle swap table transaction updates

    Args:
        data: Transaction update data
        callback: Optional callback function(swap_data)
    """
    logger.debug("Checking for swap table inserts")

    swap_data = parse_swap_insert(data)

    if swap_data:
        logger.info(f"Swap transaction: {swap_data['session_id']} - {swap_data['amount']} {swap_data['from_token']}")

        if callback:
            try:
                if asyncio.iscoroutinefunction(callback):
                    await callback(swap_data)
                else:
                    callback(swap_data)
            except Exception as e:
                logger.error(f"Swap callback error: {e}", exc_info=True)


# ============================================================================
# EXAMPLE USAGE
# ============================================================================

async def example_standalone():
    """Example: Standalone async usage with swap table subscription"""
    uri = "ws://localhost:3000/v1/database/mydb/subscribe"

    # Define swap handler
    async def on_swap(swap_data):
        print(f"New swap detected!")
        print(f"  Session: {swap_data['session_id']}")
        print(f"  From: {swap_data['from_address']}")
        print(f"  To: {swap_data['to_address']}")
        print(f"  Amount: {swap_data['amount']} {swap_data['from_token']}")

    # Define transaction handler that checks for swaps
    async def on_transaction(data):
        await handle_swap_transaction(data, on_swap)

    handlers = {"TransactionUpdate": on_transaction}

    # Connect
    ws, tasks = await connect(uri, handlers)

    try:
        # Call reducer
        result = await call_reducer(ws, "process_data", "arg1", "arg2")
        success, data = parse_reducer(result)
        logger.info(f"Reducer result: success={success}")

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

        # Keep running to receive swap updates
        await asyncio.sleep(60)

    finally:
        await disconnect(ws, tasks)


def example_flask():
    """Example: Flask integration with swap table subscription"""
    from flask import Flask, request, jsonify

    app = Flask(__name__)

    # Store recent swaps in memory
    recent_swaps = []

    # Define swap handler
    async def on_swap(swap_data):
        logger.info(f"New swap: {swap_data['session_id']} - {swap_data['amount']}")
        recent_swaps.append(swap_data)
        # Keep only last 100
        if len(recent_swaps) > 100:
            recent_swaps.pop(0)

    # Define transaction handler
    async def on_transaction(data):
        await handle_swap_transaction(data, on_swap)

    # Initialize client with swap handler
    uri = "ws://localhost:3000/v1/database/gateway2/subscribe"
    handlers = {"TransactionUpdate": on_transaction}
    stdb = FlaskSTDB(uri, handlers)
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

    @app.route("/swaps")
    def get_swaps():
        """Get recent swap transactions"""
        return jsonify({
            "total": len(recent_swaps),
            "swaps": recent_swaps[-10:]  # Last 10 swaps
        })

    @app.route("/swap/<session_id>")
    def get_swap(session_id):
        """Get swap by session_id"""
        swap = next((s for s in recent_swaps if s['session_id'] == session_id), None)
        if swap:
            return jsonify(swap)
        return jsonify({"error": "Swap not found"}), 404

    return app


# ============================================================================
# HELPER: BUILD URI
# ============================================================================

def build_uri(host: str = "localhost", port: int = 3000,
              database: str = "gateway2", secure: bool = False) -> str:
    """Build SpacetimeDB WebSocket URI"""
    proto = "wss" if secure else "ws"
    host_with_port = f"{host}:{port}" if not host.endswith("spacetimedb.com") else host
    uri = f"{proto}://{host_with_port}/v1/database/{database}/subscribe"
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
