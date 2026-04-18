"""End-to-end USSD flows exercised through the live ussdclient HTTP bridge.

Prereqs (started manually or via scripts/e2e-up.sh):
  - SpacetimeDB on 127.0.0.1:3000 with `gateway2` published
  - ussdclient_v2.py on 127.0.0.1:5000

Each test provisions its own phone number so tests can run in any order.
"""
from __future__ import annotations

import os
import subprocess
import time
import uuid
from dataclasses import dataclass

import pytest
import requests

USSD_URL = os.environ.get("USSD_URL", "http://127.0.0.1:5000/ussdeth")
DB = os.environ.get("SPACETIME_DB", "gateway2")
NETWORK = "99999"
SERVICE = "*384*6086#"
PIN = "1379"
TIMEOUT = 10.0


@dataclass
class Dial:
    session: str
    phone: str
    phone_norm: str

    def send(self, text: str) -> str:
        r = requests.post(
            USSD_URL,
            data={
                "sessionId": self.session,
                "phoneNumber": self.phone,
                "networkCode": NETWORK,
                "serviceCode": SERVICE,
                "text": text,
            },
            timeout=TIMEOUT,
        )
        r.raise_for_status()
        return r.text.strip()


def _fresh_phone() -> tuple[str, str]:
    # E.164 format; use uuid digits so concurrent runs don't collide
    digits = uuid.uuid4().int % 10**9
    phone = f"+254{digits:09d}"
    return phone, phone.lstrip("+")


def _sql(query: str) -> str:
    out = subprocess.run(
        ["spacetime", "sql", DB, query],
        capture_output=True, text=True, timeout=TIMEOUT,
    )
    return out.stdout + out.stderr


def _row_exists(table: str, where: str) -> bool:
    out = _sql(f"SELECT * FROM {table} WHERE {where}")
    return '"' in out and "Error" not in out


@pytest.fixture
def dial() -> Dial:
    phone, phone_norm = _fresh_phone()
    _sql(f"DELETE FROM esim_profile WHERE phone_number = '{phone_norm}'")
    _sql(f"DELETE FROM user_pin     WHERE phone_number = '{phone_norm}'")
    _sql(f"DELETE FROM ussd_session WHERE phone_number = '{phone}'")
    return Dial(session=f"e2e-{uuid.uuid4().hex[:8]}", phone=phone, phone_norm=phone_norm)


def test_initial_dial_new_user_sees_register_menu(dial: Dial):
    resp = dial.send("")
    assert resp.startswith("CON"), f"expected CON, got: {resp!r}"
    assert "Register" in resp, f"expected Register menu, got: {resp!r}"


def test_register_flow_creates_profile_pin_auth(dial: Dial):
    # dial in
    r = dial.send("")
    assert "Register" in r

    # pick Register
    dial.send("1")

    # enter PIN
    dial.send(f"1*{PIN}")

    # confirm PIN
    final = dial.send(f"1*{PIN}*{PIN}")
    assert "END" in final or "CON" in final, f"unexpected: {final!r}"

    # wait briefly for reducer writeback (commit is sync but query is separate)
    time.sleep(0.2)

    assert _row_exists("esim_profile", f"phone_number = '{dial.phone_norm}'"), \
        f"esim_profile not created for {dial.phone_norm}"
    assert _row_exists("user_pin", f"phone_number = '{dial.phone_norm}'"), \
        f"user_pin not created for {dial.phone_norm}"


def test_registered_user_sees_main_menu_not_register(dial: Dial):
    # Register first
    dial.send("")
    dial.send("1")
    dial.send(f"1*{PIN}")
    dial.send(f"1*{PIN}*{PIN}")

    # New session, same phone → should see MainScreen, not RegisterScreen
    second = Dial(session=f"e2e-{uuid.uuid4().hex[:8]}", phone=dial.phone, phone_norm=dial.phone_norm)
    resp = second.send("")
    assert "Send ETH" in resp or "Balance" in resp, \
        f"expected MainScreen, got: {resp!r}"


def test_send_eth_flow_creates_pending_eth_tx(dial: Dial):
    # Register first
    dial.send("")
    dial.send("1")
    dial.send(f"1*{PIN}")
    dial.send(f"1*{PIN}*{PIN}")

    # Fresh session for send-eth flow
    s = Dial(session=f"e2e-{uuid.uuid4().hex[:8]}", phone=dial.phone, phone_norm=dial.phone_norm)
    s.send("")             # MainScreen
    s.send("1")            # Send ETH → ToNumberScreen
    recv = "+254700000001"
    s.send(f"1*{recv}")    # → ToAmountScreen
    s.send(f"1*{recv}*0.01")  # → PINScreen
    confirm = s.send(f"1*{recv}*0.01*{PIN}")  # runs validate_pin, inserts eth_tx(Pending)
    assert "Confirm" in confirm or "CON" in confirm, f"unexpected: {confirm!r}"

    time.sleep(0.2)
    out = _sql(f"SELECT session_id, status FROM eth_tx WHERE session_id = '{s.session}'")
    assert s.session in out, f"no eth_tx for session {s.session}: {out}"
    assert "Pending" in out, f"eth_tx not Pending: {out}"


def test_send_eth_cancel_marks_tx_cancelled(dial: Dial):
    # register + drive send-eth to the confirm screen
    dial.send(""); dial.send("1"); dial.send(f"1*{PIN}"); dial.send(f"1*{PIN}*{PIN}")
    s = Dial(session=f"e2e-{uuid.uuid4().hex[:8]}", phone=dial.phone, phone_norm=dial.phone_norm)
    s.send("")
    s.send("1")
    recv = "+254700000002"
    s.send(f"1*{recv}")
    s.send(f"1*{recv}*0.01")
    s.send(f"1*{recv}*0.01*{PIN}")  # Pending row created

    # Cancel
    s.send(f"1*{recv}*0.01*{PIN}*2")
    time.sleep(0.2)

    out = _sql(f"SELECT status FROM eth_tx WHERE session_id = '{s.session}'")
    assert "Cancelled" in out, f"expected Cancelled, got: {out}"


def test_send_eth_confirm_marks_tx_submitted(dial: Dial):
    dial.send(""); dial.send("1"); dial.send(f"1*{PIN}"); dial.send(f"1*{PIN}*{PIN}")
    s = Dial(session=f"e2e-{uuid.uuid4().hex[:8]}", phone=dial.phone, phone_norm=dial.phone_norm)
    s.send("")
    s.send("1")
    recv = "+254700000003"
    s.send(f"1*{recv}")
    s.send(f"1*{recv}*0.01")
    s.send(f"1*{recv}*0.01*{PIN}")

    # Confirm
    s.send(f"1*{recv}*0.01*{PIN}*1")
    time.sleep(0.2)

    out = _sql(f"SELECT status FROM eth_tx WHERE session_id = '{s.session}'")
    assert "Submitted" in out, f"expected Submitted, got: {out}"


def test_bad_pin_rejects(dial: Dial):
    dial.send(""); dial.send("1"); dial.send(f"1*{PIN}"); dial.send(f"1*{PIN}*{PIN}")
    s = Dial(session=f"e2e-{uuid.uuid4().hex[:8]}", phone=dial.phone, phone_norm=dial.phone_norm)
    s.send(""); s.send("1")
    recv = "+254700000004"
    s.send(f"1*{recv}")
    s.send(f"1*{recv}*0.01")
    resp = s.send(f"1*{recv}*0.01*9999")  # wrong PIN
    assert "Invalid PIN" in resp, f"bad PIN should show 'Invalid PIN', got: {resp!r}"


def test_invalid_phone_format_rejects(dial: Dial):
    dial.send(""); dial.send("1"); dial.send(f"1*{PIN}"); dial.send(f"1*{PIN}*{PIN}")
    s = Dial(session=f"e2e-{uuid.uuid4().hex[:8]}", phone=dial.phone, phone_norm=dial.phone_norm)
    s.send(""); s.send("1")
    resp = s.send("1*not-a-phone")
    assert "Invalid phone number format" in resp, \
        f"invalid phone should show format error, got: {resp!r}"
