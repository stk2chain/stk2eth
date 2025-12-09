import asyncio
import json
import sys
import logging
import uuid
import websockets
from stdb_client import (
    build_uri,
    connect,
    disconnect,
    send_subscription,
    configure_logging
)

# Configure logging based on command line args
log_level = logging.DEBUG if "--debug" in sys.argv else logging.INFO
configure_logging(level=log_level)

logger = logging.getLogger(__name__)


# Initialize SpacetimeDB client
uri = build_uri(host="0.0.0.0", port=3000, database="gateway2")

logger.info("="*60)
logger.info("  SpacetimeDB ETH Client Starting")
logger.info("="*60)



async def ethclient():
    
    # Optional: Define message handlers
    async def on_transaction(tx_data):
        logger.info(f"Transaction handler called: {tx_data}")
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

    
    handlers = {"TransactionUpdate": on_transaction}
    
    # Connect
    ws, tasks = await connect(uri, handlers)
    
    try:
        # Send subscription
        await send_subscription(ws, "swap")
        
        # Call reducer
        # result = await call_reducer(ws, "process_data", "arg1", "arg2")
        # success, data = parse_reducer(result)
        # logger.info(f"Reducer result: success={success}, data={data}")
        
        # # Execute query
        # result = await query(ws, "SELECT * FROM users")
        # success, rows = parse_query(result)
        # if success:
        #     logger.info(f"Query returned {len(rows)} rows")
            
        #     # Filter results
        #     active = filter_rows(rows, status="active")
        #     logger.info(f"Active users: {len(active)}")
            
        #     # Find specific row
        #     user = find_one(rows, name="Alice")
        #     logger.info(f"Found user: {user}")
        
        # Keep running
        await asyncio.sleep(60)
        
    except Exception as e:
        logger.error(f"Error: {e}")


if __name__ == '__main__':
    asyncio.run(ethclient())