"""
Minimal USSD Gateway using functional SpacetimeDB client

Usage:
    python app.py
    
    # With debug logging:
    python app.py --debug
"""
import os
import sys
import logging
from flask import Flask, request
from flask_cors import CORS
from stdb_client import (
    FlaskSTDB, 
    build_uri, 
    parse_query, 
    configure_logging
)

# Configure logging based on command line args
log_level = logging.DEBUG if "--debug" in sys.argv else logging.INFO
configure_logging(level=log_level)

logger = logging.getLogger(__name__)

# Create Flask app
app = Flask(__name__)
CORS(app)


stdb_host = os.getenv("SPACETIMEDB_HOST", "0.0.0.0")            
stdb_dbname = os.getenv("SPACETIMEDB_DBNAME", "gateway2")
stdb_port = int(os.getenv("SPACETIMEDB_PORT", "3000"))

# Initialize SpacetimeDB client
uri = build_uri(host=stdb_host, port=stdb_port, database=stdb_dbname, secure=True)
stdb = FlaskSTDB(uri)

logger.info("="*60)
logger.info("  SpacetimeDB USSD Gateway Starting")
logger.info("="*60)

stdb.start()


@app.route("/ussdeth", methods=['POST'])
def ussd():
    """Handle USSD requests"""
    session_id = request.values.get("sessionId")
    phone = request.values.get("phoneNumber")
    network = request.values.get("networkCode")
    service = request.values.get("serviceCode")
    text = request.values.get("text", "default")
    
    logger.info("="*50)
    logger.info(f"USSD Request Received")
    # logger.info(f"  Session ID: {session_id}")
    # logger.info(f"  Phone: {phone}")
    # logger.info(f"  Network: {network}")
    # logger.info(f"  Service: {service}")
    # logger.info(f"  Text: {text}")
    logger.info("="*50)
    
    try:
        # logger.info("Step 0/2: Subscribing to swap")
        # stdb.send_subscription("swap", timeout=10)
        # logger.info("  ✓ Subscription sent is ready")
        # Step 1: Process USSD input via reducer
        logger.info("Step 1/2: Processing USSD input via reducer")
        stdb.call_reducer("process_ussd_step", session_id, phone, network, service, text)
        logger.info("  ✓ Reducer executed successfully")
        
        # Step 2: Query for response
        logger.info("Step 2/2: Querying for USSD response")
        sql = f"SELECT * FROM ussd_response WHERE session_id = '{session_id}'"
        result = stdb.query(sql)
        success, rows = parse_query(result)
        
        if not success:
            logger.error(f"  ✗ Query error: {rows}")
            return "END Database error"
        
        if not rows:
            logger.warning("  ✗ No response found in database")
            return "END No response"
        
        # Return the latest response
        response_text = rows[-1].get("response_text", "END No response")
        logger.info(f"  ✓ Response retrieved: {response_text[:50]}{'...' if len(response_text) > 50 else ''}")
        logger.info("="*50)
        
        return response_text
        
    except TimeoutError:
        logger.error("✗ Request timeout")
        return "END Request timeout"
    except ConnectionError as e:
        logger.error(f"✗ Connection error: {e}")
        return "END Service unavailable"
    except Exception as e:
        logger.error(f"✗ Unexpected error: {e}", exc_info=True)
        return "END Service error"


@app.route("/health")
def health():
    """Health check"""
    is_connected = stdb.ready.is_set()
    status = "healthy" if is_connected else "connecting"
    
    logger.debug(f"Health check: {status}")
    
    return {
        "status": status,
        "connected": is_connected
    }, 200 if is_connected else 503


@app.route("/stats")
def stats():
    """Statistics endpoint"""
    is_connected = stdb.ready.is_set()
    
    return {
        "connected": is_connected,
        "uri": stdb.uri,
        "pending_requests": len(stdb.ws._pending) if stdb.ws else 0
    }


if __name__ == '__main__':
    logger.info("\n" + "="*60)
    logger.info("  Flask HTTP Server Starting")
    logger.info("  Host: 0.0.0.0:5000")
    logger.info("  Debug Mode: ON" if "--debug" in sys.argv else "  Production Mode")
    logger.info("="*60 + "\n")
    
    app.run(host='0.0.0.0', port=5000, debug=False, use_reloader=False)