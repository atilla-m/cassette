"""Structural safety checks for Tauri's base64-encoded Minisign envelope.

Cryptographic verification belongs to update-verifier, using the same verifier
as tauri-plugin-updater; these checks alone never authorize publication.
"""
import base64
import re
from pathlib import Path


def validate_signature(path: Path, version: str) -> str:
    image_name = f"Cassette_{version}_amd64.AppImage"
    if path.name != image_name + ".sig" or path.is_symlink():
        raise ValueError("Signature filename does not exactly match the versioned AppImage")
    if not path.is_file() or not 1 <= path.stat().st_size <= 4096:
        raise ValueError("Signature must be a regular nonempty file of at most 4096 bytes")
    image = path.with_suffix("")
    if image.is_symlink() or not image.is_file():
        raise ValueError("Signature has no exact adjacent AppImage pair")
    raw = path.read_bytes().strip()
    try:
        decoded = base64.b64decode(raw, validate=True)
        if base64.b64encode(decoded) != raw:
            raise ValueError("Noncanonical signature encoding")
        lines = decoded.decode("ascii").splitlines()
        if len(lines) != 4 or lines[0] != "untrusted comment: signature from tauri secret key":
            raise ValueError("Unexpected Tauri signature envelope")
        signature = base64.b64decode(lines[1], validate=True)
        global_signature = base64.b64decode(lines[3], validate=True)
        if len(signature) != 74 or signature[:2] not in (b"Ed", b"ED") or len(global_signature) != 64:
            raise ValueError("Malformed Minisign signature packet")
        secret_pattern = rb"-----BEGIN (?:[A-Z ]+)?PRIVATE KEY-----|AKIA[0-9A-Z]{16}|gh[pousr]_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|xox[baprs]-[A-Za-z0-9-]{20,}"
        if any(re.search(secret_pattern, packet) for packet in (decoded, signature, global_signature)):
            raise ValueError("Credential material in signature")
        if not re.fullmatch(r"trusted comment: timestamp:[0-9]+\tfile:" + re.escape(image_name), lines[2]):
            raise ValueError("Signature trusted filename does not match AppImage")
    except (UnicodeError, ValueError) as error:
        raise ValueError("Malformed, secret-containing, or incorrectly paired Tauri signature") from error
    return raw.decode("ascii")
