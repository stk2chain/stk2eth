import json
import logging
import sys
import time
from eth_account import Account
from eth_utils import keccak
from stdb_client_v5 import build_uri, FlaskSTDB, configure_logging

from scripts.core.wallet import create_phone_burner_wallet

configure_logging(logging.DEBUG if "--debug" in sys.argv else logging.INFO)
logger = logging.getLogger(__name__)

# ============================================================================
# PURE FUNCTIONS - WALLET
# ============================================================================

gen_seed = lambda phone, chain=1: keccak(text=f"phone_wallet_{phone}_{chain}")
gen_wallet = lambda phone: (lambda a: {'address': a.address, 'key': a.key.hex(), 'phone': phone})(Account.from_key(gen_seed(phone)))

# ============================================================================
# PURE FUNCTIONS
# ============================================================================

parse_insert = lambda insert: json.loads(insert)
extract_tables = lambda tx: tx.get('TransactionUpdate', {}).get('status', {}).get('Committed', {}).get('tables', [])
is_committed = lambda tx: 'Committed' in tx.get('TransactionUpdate', {}).get('status', {})
get_table_name = lambda table: table.get('table_name')
get_inserts = lambda table: [i for u in table.get('updates', []) for i in u.get('inserts', [])]

# ============================================================================
# TABLE HANDLERS
# ============================================================================

def handle_esim(insert, stdb):
    """esim_profile: phone → wallet → save"""
    logger.debug(f"handle_esim() called: {len(insert)} bytes")
    
    data = parse_insert(insert)
    phone_number, wallet_address = data[0], data[1]
    
    logger.info(f"✓ eSIM profile: {phone_number}")
    logger.debug(f"  Wallet: {wallet_address}")
    
    if wallet_address == "":
        wallet, signed_auth, phone_salt = create_phone_burner_wallet(phone_number, 0)
        logger.info(f"  Wallet: {wallet}")
        logger.debug(f"  Auth: {signed_auth}")
        logger.debug(f"  Phone Salt: {phone_salt}")
        
        try:
            logger.debug(f"  → Calling map_phone_to_wallet")
            stdb.call_reducer("map_phone_to_wallet", phone_number, wallet)
            logger.info(f"  ✓ Saved")
        except Exception as e:
            logger.error(f"  ✗ Save failed: {e}")

def handle_swap(insert, stdb):
    """swap: data → parse → log (TODO: execute transfer)"""
    logger.debug(f"handle_swap() called: {len(insert)} bytes")
    
    data = parse_insert(insert)
    swap = {
        'id': data[0], 'session_id': data[1],
        'from_address': data[2], 'to_address': data[3],
        'amount': data[4], 'from_token': data[5], 'to_token': data[6]
    }
    
    logger.info(f"✓ Swap: {swap['id']}")
    logger.debug(f"  Session: {swap['session_id']}")
    logger.debug(f"  Amount: {swap['amount']}")
    logger.debug(f"  From: {swap['from_address']}")
    logger.debug(f"  To: {swap['to_address']}")
    logger.debug(f"  Token: {swap['from_token']}")
    
    # TODO: Execute Permit2 transfer
    logger.debug(f"  TODO: Execute Permit2 transfer")

# ============================================================================
# ROUTING
# ============================================================================

def route_tx(tx_data, stdb, handlers):
    """Route transaction to appropriate handler"""
    logger.debug(f"route_tx() called")
    
    if not is_committed(tx_data):
        logger.debug(f"  Not committed, skipping")
        return
    
    tables = extract_tables(tx_data)
    logger.debug(f"  Tables: {[get_table_name(t) for t in tables]}")
    
    for table in tables:
        name = get_table_name(table)
        handler = handlers.get(name)
        
        if handler:
            logger.debug(f"  → Routing to {name} handler")
            inserts = get_inserts(table)
            logger.debug(f"    Inserts: {len(inserts)}")
            
            for insert in inserts:
                try:
                    handler(insert, stdb)
                except Exception as e:
                    logger.error(f"  ✗ Handler error ({name}): {e}", exc_info=True)
        else:
            logger.debug(f"  No handler for {name}")


# ============================================================================
# HELPERS
# ============================================================================

def wait_ready(stdb, timeout=10):
    """Wait for client to be ready"""
    start = time.time()
    while not stdb.ready.is_set() and time.time() - start < timeout:
        time.sleep(0.1)
    return stdb.ready.is_set()

# ============================================================================
# MAIN
# ============================================================================

def run():
    logger.info("="*60)
    logger.info("  SpacetimeDB ETH Client")
    logger.info("="*60)
    
    uri = build_uri(host="0.0.0.0", port=3000, database="gateway2")
    logger.info(f"URI: {uri}")
    
    # Table handlers map
    handlers = {
        'esim_profile': handle_esim,
        'swap': handle_swap
    }
    logger.debug(f"Handlers: {list(handlers.keys())}")
    
    # Init client
    stdb = FlaskSTDB(uri, handlers={})
    stdb.start()
    logger.debug("Client started, waiting for ready...")
    
    if not wait_ready(stdb):
        logger.error("✗ Timeout waiting for ready")
        return
    
    logger.info("✓ Ready")
    
    # Subscribe
    for table in handlers.keys():
        logger.debug(f"→ Subscribing to {table}")
        try:
            stdb.subscribe(table)
            logger.info(f"✓ Subscribed: {table}")
        except Exception as e:
            logger.error(f"✗ Subscribe failed ({table}): {e}")
            return
    
    # Register transaction router
    stdb.state['handlers']['TransactionUpdate'] = lambda tx: route_tx(tx, stdb, handlers)
    logger.debug("Transaction router registered")
    
    logger.info("="*60)
    logger.info("  Listening...")
    logger.info("="*60)
    
    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        logger.info("\n✓ Shutdown")

if __name__ == '__main__':
    run()