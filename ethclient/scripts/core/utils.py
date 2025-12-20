from typing import Optional
import hashlib
import re

def normalize_phone_number(phone: str) -> str:
    """
    Normalizes a phone number by removing all non-digit characters.

    Arguments:
        phone: Phone number in any format (e.g., "+1-555-123-4567", "(555) 123-4567")

    Returns:
        Normalized phone number with only digits
    """
    digits_only = re.sub(r'\D', '', phone)
    return digits_only


def phone_to_salt(phone_number: str, user_salt: Optional[str] = None) -> bytes:
    """
    Converts a phone number (and optional user salt) into a salt for Nick's method.

    Arguments:
        phone_number: Phone number (will be normalized)
        user_salt: Optional additional user-provided salt

    Returns:
        32-byte salt derived from phone number
    """
    normalized_phone = normalize_phone_number(phone_number)
    print(f"Normalized phone: {normalized_phone}")

    # Combine phone with optional user salt
    if user_salt:
        combined = f"{normalized_phone}||{user_salt}".encode('utf-8')
    else:
        combined = normalized_phone.encode('utf-8')

    # Hash to get 32-byte salt
    salt = hashlib.sha256(combined).digest()
    return salt

