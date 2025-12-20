"""
Minimal SpacetimeDB Client - Pure Functional with aiohttp
Connection persistence, auto-reconnect, subscription recovery
"""
import asyncio
import json
import uuid
import logging
from typing import Dict, Callable, Any
from functools import partial
import aiohttp

logger = logging.getLogger(__name__)

# ============================================================================
# PURE FUNCTIONS
# ============================================================================

build_uri = lambda host="localhost", port=3000, database="gateway2", secure=False: (
    f"{'wss' if secure else 'ws'}://"
    f"{host if host.endswith('spacetimedb.com') else f'{host}:{port}'}"
    f"/v1/database/{database}/subscribe"
)

get_msg_type = lambda d: next((k for k in ['IdentityToken', 'TransactionUpdate', 'OneOffQueryResponse', 'SubscriptionUpdate', 'SubscribeApplied', 'SubscriptionError'] if k in d), 'Unknown')

parse_json = lambda msg: json.loads(msg) if isinstance(msg, str) else msg

make_reducer_msg = lambda name, args, req_id: {
    "CallReducer": {"reducer": name, "args": json.dumps(list(args)), "request_id": req_id, "flags": 0}
}

make_query_msg = lambda sql, msg_id: {
    "OneOffQuery": {"message_id": msg_id, "query_string": sql}
}

make_subscription_msg = lambda table, req_id: {
    "SubscribeSingle": {
        "query": f"SELECT * FROM {table}",
        "request_id": req_id,
        "query_id": {"id": int(uuid.uuid4().int & 0xFFFFFFFF)}
    }
}

extract_query_rows = lambda resp: (
    [parse_json(r) for r in resp.get('OneOffQueryResponse', {}).get('tables', [{}])[0].get('rows', [])]
)

extract_query_error = lambda resp: resp.get('OneOffQueryResponse', {}).get('error', {}).get('some')

is_committed = lambda resp: 'Committed' in resp.get('TransactionUpdate', {}).get('status', {})
is_failed = lambda resp: 'Failed' in resp.get('TransactionUpdate', {}).get('status', {})
get_failure = lambda resp: resp.get('TransactionUpdate', {}).get('status', {}).get('Failed')

filter_rows = lambda rows, **f: [r for r in rows if all(r.get(k) == v for k, v in f.items())]
find_one = lambda rows, **f: (lambda m: m[0] if m else None)(filter_rows(rows, **f))

# ============================================================================
# STATE MANAGEMENT (Minimal)
# ============================================================================

def make_state():
    """Create connection state"""
    return {
        'ws': None,
        'session': None,
        'pending': {},
        'subscriptions': set(),
        'handlers': {},
        'req_counter': 0,
        'reconnecting': False
    }

next_req_id = lambda state: (state.update({'req_counter': state['req_counter'] + 1}), state['req_counter'])[1]

# ============================================================================
# CONNECTION
# ============================================================================

async def connect(uri: str, handlers: Dict[str, Callable], state: dict) -> aiohttp.ClientWebSocketResponse:
    """Connect with auto-reconnect"""
    logger.debug(f"connect() called: reconnecting={state['reconnecting']}")

    if state['reconnecting']:
        logger.debug("Already reconnecting, returning existing ws")
        return state['ws']

    state['reconnecting'] = True
    logger.debug("Set reconnecting=True")

    try:
        if not state['session']:
            logger.debug("Creating new aiohttp ClientSession")
            state['session'] = aiohttp.ClientSession()
        else:
            logger.debug("Reusing existing ClientSession")

        logger.info(f"→ Connecting to: {uri}")
        logger.debug(f"  Protocol: v1.json.spacetimedb")
        logger.debug(f"  Max message size: 10MB")
        logger.debug(f"  Heartbeat: 20s")

        ws = await state['session'].ws_connect(
            uri,
            protocols=['v1.json.spacetimedb'],
            max_msg_size=10 * 1024 * 1024,
            heartbeat=20
        )

        logger.debug(f"WebSocket object created: {ws}")
        logger.debug(f"  Closed: {ws.closed}")
        logger.debug(f"  Protocol: {ws.protocol}")

        state['ws'] = ws
        state['handlers'] = handlers
        logger.info("✓ Connected successfully")
        logger.debug(f"  Handlers registered: {list(handlers.keys())}")

        # Recover subscriptions
        await recover_subscriptions(state)

        return ws

    except Exception as e:
        logger.error(f"✗ Connection failed: {e}", exc_info=True)
        raise
    finally:
        state['reconnecting'] = False
        logger.debug("Set reconnecting=False")

async def recover_subscriptions(state: dict):
    """Resubscribe to all tables after reconnect"""
    logger.debug(f"recover_subscriptions() called: {len(state['subscriptions'])} subscriptions")

    if not state['subscriptions']:
        logger.debug("No subscriptions to recover")
        return

    logger.info(f"→ Recovering {len(state['subscriptions'])} subscriptions")
    logger.debug(f"  Tables: {list(state['subscriptions'])}")

    for table in state['subscriptions']:
        try:
            req_id = next_req_id(state)
            msg = make_subscription_msg(table, req_id)

            logger.debug(f"  → Resubscribing to '{table}'")
            logger.debug(f"    request_id: {req_id}")
            logger.debug(f"    message: {json.dumps(msg)}")

            await state['ws'].send_json(msg)
            logger.info(f"  ✓ Resubscribed: {table}")

        except Exception as e:
            logger.error(f"  ✗ Failed to resubscribe '{table}': {e}", exc_info=True)

async def ensure_connected(uri: str, state: dict):
    """Ensure connection is alive, reconnect if needed"""
    ws_closed = state['ws'].closed if state['ws'] else True

    logger.debug(f"ensure_connected() called: ws={state['ws'] is not None}, closed={ws_closed}")

    if not state['ws'] or ws_closed:
        logger.warning("⚠ Connection lost, initiating reconnect...")
        logger.debug(f"  ws exists: {state['ws'] is not None}")
        logger.debug(f"  ws closed: {ws_closed}")
        await connect(uri, state['handlers'], state)
        logger.debug("✓ Reconnection completed")
    else:
        logger.debug("Connection alive, no action needed")

# ============================================================================
# MESSAGE HANDLING
# ============================================================================

async def handle_messages(uri: str, state: dict):
    """Message loop with auto-reconnect"""
    msg_count = 0
    reconnect_count = 0

    logger.info("→ Starting message handler loop")

    while True:
        try:
            logger.debug(f"Loop iteration {reconnect_count + 1}")
            await ensure_connected(uri, state)
            ws = state['ws']

            logger.debug(f"Entering message receive loop (ws={ws})")

            async for msg in ws:
                msg_count += 1

                if msg.type == aiohttp.WSMsgType.TEXT:
                    logger.debug(f"← Message #{msg_count} received ({len(msg.data)} bytes)")
                    logger.debug(f"  Raw data: {msg.data[:200]}...")
                    await process_message(parse_json(msg.data), state)

                elif msg.type == aiohttp.WSMsgType.ERROR:
                    logger.error(f"✗ WebSocket error: {ws.exception()}")
                    break

                elif msg.type == aiohttp.WSMsgType.CLOSE:
                    logger.warning(f"← Close frame received: code={msg.data}, extra={msg.extra}")
                    break

                else:
                    logger.debug(f"← Unknown message type: {msg.type}")

            # Connection closed, reconnect
            reconnect_count += 1
            logger.warning(f"⚠ Connection closed, reconnecting in 5s... (attempt #{reconnect_count})")
            logger.debug(f"  Messages processed this session: {msg_count}")
            await asyncio.sleep(5)

        except asyncio.CancelledError:
            logger.info("✓ Message handler cancelled gracefully")
            break
        except Exception as e:
            logger.error(f"✗ Message handler error: {e}", exc_info=True)
            logger.debug(f"  Reconnecting in 5s...")
            await asyncio.sleep(5)

async def process_message(data: dict, state: dict):
    """Process single message"""
    msg_type = get_msg_type(data)
    logger.debug(f"process_message() called: type={msg_type}")
    logger.debug(f"  Data keys: {list(data.keys())}")

    # Route to handlers
    handler = state['handlers'].get(msg_type)
    if handler:
        logger.debug(f"  → Routing to handler for '{msg_type}'")
        await safe_call(handler, data)
    else:
        logger.debug(f"  No handler registered for '{msg_type}'")
        logger.debug(f"  Available handlers: {list(state['handlers'].keys())}")

    # Resolve pending
    logger.debug(f"  Checking pending requests ({len(state['pending'])} pending)")
    resolved = resolve_pending(data, state['pending'])
    if resolved:
        logger.debug(f"  ✓ Resolved: {resolved}")

def resolve_pending(data: dict, pending: dict) -> str:
    """Resolve pending futures"""
    logger.debug(f"resolve_pending() called: {len(pending)} pending requests")

    # Transaction with request_id
    if 'TransactionUpdate' in data:
        tx = data['TransactionUpdate']
        req_id = tx.get('request_id')
        status = tx.get('status', {})

        logger.debug(f"  TransactionUpdate: request_id={req_id}, status={list(status.keys())}")

        if req_id in pending:
            logger.debug(f"  Found pending request: {req_id}")
            if 'Committed' in status:
                logger.debug(f"  ✓ Resolving as committed")
                pending[req_id].set_result(data)
                return f"request_id={req_id} (committed)"
            elif 'Failed' in status:
                error = get_failure(data)
                logger.warning(f"  ✗ Resolving as failed: {error}")
                pending[req_id].set_exception(Exception(error))
                return f"request_id={req_id} (failed)"

        # Fallback: resolve oldest
        elif pending and ('Committed' in status or 'Failed' in status):
            oldest = min(k for k in pending.keys() if isinstance(k, int))
            logger.debug(f"  No matching request_id, resolving oldest: {oldest}")

            if 'Committed' in status:
                logger.debug(f"  ✓ Resolving oldest as committed")
                pending[oldest].set_result(data)
                return f"oldest={oldest} (committed)"
            else:
                error = get_failure(data)
                logger.warning(f"  ✗ Resolving oldest as failed: {error}")
                pending[oldest].set_exception(Exception(error))
                return f"oldest={oldest} (failed)"

    # Query with message_id
    elif 'OneOffQueryResponse' in data:
        msg_id = data['OneOffQueryResponse'].get('message_id')
        logger.debug(f"  OneOffQueryResponse: message_id={msg_id}")

        if msg_id in pending:
            logger.debug(f"  ✓ Resolving query: {msg_id}")
            pending[msg_id].set_result(data)
            return f"message_id={msg_id}"
        else:
            logger.debug(f"  Query message_id not in pending")

    logger.debug(f"  No pending request resolved")
    return None

async def safe_call(fn: Callable, *args):
    """Safe handler call"""
    logger.debug(f"safe_call() executing: {fn.__name__ if hasattr(fn, '__name__') else fn}")
    try:
        if asyncio.iscoroutinefunction(fn):
            logger.debug(f"  Awaiting async function")
            await fn(*args)
        else:
            logger.debug(f"  Calling sync function")
            fn(*args)
        logger.debug(f"  ✓ Handler completed successfully")
    except Exception as e:
        logger.error(f"✗ Handler error: {e}", exc_info=True)

# ============================================================================
# OPERATIONS
# ============================================================================

async def call_reducer(state: dict, name: str, *args, timeout: float = 10) -> dict:
    """Call reducer"""
    req_id = next_req_id(state)
    logger.info(f"→ Calling reducer: {name}")
    logger.debug(f"  request_id: {req_id}")
    logger.debug(f"  args: {args}")
    logger.debug(f"  timeout: {timeout}s")

    future = asyncio.get_event_loop().create_future()
    state['pending'][req_id] = future
    logger.debug(f"  Future created and registered")

    try:
        msg = make_reducer_msg(name, args, req_id)
        logger.debug(f"  Message: {json.dumps(msg)}")

        await state['ws'].send_json(msg)
        logger.debug(f"  ✓ Message sent, waiting for response...")

        result = await asyncio.wait_for(future, timeout)
        logger.info(f"← Reducer response received: {name}")
        logger.debug(f"  Result: {result}")
        return result

    except asyncio.TimeoutError:
        logger.error(f"✗ Reducer timeout: {name} (after {timeout}s)")
        raise
    except Exception as e:
        logger.error(f"✗ Reducer error: {name} - {e}", exc_info=True)
        raise
    finally:
        state['pending'].pop(req_id, None)
        logger.debug(f"  Cleaned up request_id: {req_id}")

async def query(state: dict, sql: str, timeout: float = 10) -> dict:
    """Execute query"""
    msg_id = uuid.uuid4().hex
    logger.info(f"→ Executing query")
    logger.debug(f"  message_id: {msg_id}")
    logger.debug(f"  SQL: {sql[:100]}{'...' if len(sql) > 100 else ''}")
    logger.debug(f"  timeout: {timeout}s")

    future = asyncio.get_event_loop().create_future()
    state['pending'][msg_id] = future
    logger.debug(f"  Future created and registered")

    try:
        msg = make_query_msg(sql, msg_id)
        logger.debug(f"  Message: {json.dumps(msg)}")

        await state['ws'].send_json(msg)
        logger.debug(f"  ✓ Query sent, waiting for response...")

        result = await asyncio.wait_for(future, timeout)
        logger.info(f"← Query response received")
        logger.debug(f"  Result keys: {list(result.keys())}")
        return result

    except asyncio.TimeoutError:
        logger.error(f"✗ Query timeout (after {timeout}s)")
        raise
    except Exception as e:
        logger.error(f"✗ Query error: {e}", exc_info=True)
        raise
    finally:
        state['pending'].pop(msg_id, None)
        logger.debug(f"  Cleaned up message_id: {msg_id}")

async def subscribe(state: dict, table: str):
    """Subscribe to table"""
    req_id = next_req_id(state)
    logger.info(f"→ Subscribing to table: {table}")
    logger.debug(f"  request_id: {req_id}")

    state['subscriptions'].add(table)
    logger.debug(f"  Added to subscriptions set (total: {len(state['subscriptions'])})")
    logger.debug(f"  All subscriptions: {list(state['subscriptions'])}")

    msg = make_subscription_msg(table, req_id)
    logger.debug(f"  Message: {json.dumps(msg)}")

    await state['ws'].send_json(msg)
    logger.info(f"✓ Subscribed: {table}")
    logger.debug(f"  Subscription message sent successfully")

# ============================================================================
# FLASK INTEGRATION
# ============================================================================

class FlaskSTDB:
    """Flask integration with connection persistence"""

    def __init__(self, uri: str, handlers: dict = None):
        self.uri = uri
        self.state = make_state()
        self.state['handlers'] = handlers or {}
        self.loop = None
        self.ready = asyncio.Event()

    def start(self):
        """Start background client"""
        import threading

        logger.info("="*60)
        logger.info("  Starting FlaskSTDB background client")
        logger.info("="*60)
        logger.debug(f"  URI: {self.uri}")
        logger.debug(f"  Handlers: {list(self.state['handlers'].keys())}")

        def run():
            logger.debug("Background thread starting...")
            self.loop = asyncio.new_event_loop()
            asyncio.set_event_loop(self.loop)
            logger.debug(f"  Event loop created: {self.loop}")

            try:
                logger.debug("  Running background client...")
                self.loop.run_until_complete(self._run())
            except Exception as e:
                logger.error(f"✗ Background thread error: {e}", exc_info=True)

        thread = threading.Thread(target=run, daemon=True, name="STDB-Client")
        thread.start()
        logger.info(f"✓ Background thread started: {thread.name}")
        logger.debug(f"  Thread ID: {thread.ident}")
        logger.debug(f"  Is daemon: {thread.daemon}")

    async def _run(self):
        """Background client loop"""
        logger.debug("_run() called - connecting...")
        await connect(self.uri, self.state['handlers'], self.state)

        self.ready.set()
        logger.info("="*60)
        logger.info("  ✓ FlaskSTDB client ready")
        logger.info("="*60)
        logger.debug(f"  WebSocket: {self.state['ws']}")
        logger.debug(f"  State keys: {list(self.state.keys())}")

        logger.debug("Starting message handler...")
        await handle_messages(self.uri, self.state)

    def subscribe(self, table: str, timeout: float = 5):
        """Subscribe to table (blocking)"""
        logger.info(f"FlaskSTDB.subscribe() called: {table}")
        logger.debug(f"  Timeout: {timeout}s")
        logger.debug(f"  Ready: {self.ready.is_set()}")

        if not self.ready.is_set():
            logger.error("✗ Client not ready")
            raise ConnectionError("Client not ready")

        logger.debug(f"  Submitting to event loop: {self.loop}")

        future = asyncio.run_coroutine_threadsafe(
            subscribe(self.state, table),
            self.loop
        )

        logger.debug("  Waiting for future result...")
        try:
            result = future.result(timeout)
            logger.info(f"✓ Subscription completed: {table}")
            logger.debug(f"  Result: {result}")
            return result
        except Exception as e:
            logger.error(f"✗ Subscription failed: {e}", exc_info=True)
            raise

    def call_reducer(self, name: str, *args, timeout: float = 10) -> dict:
        """Call reducer (blocking)"""
        logger.info(f"FlaskSTDB.call_reducer() called: {name}")
        logger.debug(f"  Args: {args}")
        logger.debug(f"  Timeout: {timeout}s")
        logger.debug(f"  Ready: {self.ready.is_set()}")

        if not self.ready.is_set():
            logger.error("✗ Client not ready")
            raise ConnectionError("Client not ready")

        logger.debug(f"  Submitting to event loop: {self.loop}")

        future = asyncio.run_coroutine_threadsafe(
            call_reducer(self.state, name, *args, timeout=timeout),
            self.loop
        )

        logger.debug("  Waiting for future result...")
        try:
            result = future.result(timeout)
            logger.info(f"✓ Reducer call completed: {name}")
            logger.debug(f"  Result keys: {list(result.keys()) if isinstance(result, dict) else type(result)}")
            return result
        except Exception as e:
            logger.error(f"✗ Reducer call failed: {e}", exc_info=True)
            raise

    def query(self, sql: str, timeout: float = 10) -> dict:
        """Query (blocking)"""
        logger.info(f"FlaskSTDB.query() called")
        logger.debug(f"  SQL: {sql[:100]}{'...' if len(sql) > 100 else ''}")
        logger.debug(f"  Timeout: {timeout}s")
        logger.debug(f"  Ready: {self.ready.is_set()}")

        if not self.ready.is_set():
            logger.error("✗ Client not ready")
            raise ConnectionError("Client not ready")

        logger.debug(f"  Submitting to event loop: {self.loop}")

        future = asyncio.run_coroutine_threadsafe(
            query(self.state, sql, timeout=timeout),
            self.loop
        )

        logger.debug("  Waiting for future result...")
        try:
            result = future.result(timeout)
            logger.info(f"✓ Query completed")
            logger.debug(f"  Result keys: {list(result.keys()) if isinstance(result, dict) else type(result)}")
            return result
        except Exception as e:
            logger.error(f"✗ Query failed: {e}", exc_info=True)
            raise

    def parse_query(self, resp: dict) -> tuple:
        """Parse query response"""
        error = extract_query_error(resp)
        if error:
            return False, error
        return True, extract_query_rows(resp)

    def parse_reducer(self, resp: dict) -> tuple:
        """Parse reducer response"""
        if is_failed(resp):
            return False, get_failure(resp)
        if is_committed(resp):
            return True, resp
        return False, "Unknown status"

# ============================================================================
# CONFIGURATION
# ============================================================================

def configure_logging(level=logging.INFO):
    """Configure logging"""
    logging.basicConfig(
        level=level,
        format='%(asctime)s [%(levelname)s] %(name)s: %(message)s',
        datefmt='%H:%M:%S'
    )
    logger.setLevel(level)

# ============================================================================
# STANDALONE USAGE
# ============================================================================

async def run_standalone(uri: str, handlers: dict):
    """Standalone async usage"""
    state = make_state()
    state['handlers'] = handlers

    # Connect
    await connect(uri, handlers, state)

    # Subscribe
    await subscribe(state, "esim_profile")

    # Start message handler
    task = asyncio.create_task(handle_messages(uri, state))

    try:
        await task
    except KeyboardInterrupt:
        task.cancel()
        await state['session'].close()

# ============================================================================
# EXAMPLE
# ============================================================================

if __name__ == "__main__":
    configure_logging(logging.DEBUG)

    # Example: Standalone
    async def on_tx(data):
        logger.info(f"Transaction: {get_msg_type(data)}")

    uri = build_uri("0.0.0.0", 3000, "gateway2")
    asyncio.run(run_standalone(uri, {"TransactionUpdate": on_tx}))
