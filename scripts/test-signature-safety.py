"""Structural fixtures only: no keypair is generated and nothing is signed."""
import base64
import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path
from update_signature import validate_signature

VERSION = "0.1.0-beta.1"
NAME = f"Cassette_{VERSION}_amd64.AppImage"


def envelope(name=NAME):
    # Structurally valid but deliberately NOT cryptographically valid.
    packet = base64.b64encode(b"ED" + bytes(72)).decode()
    global_packet = base64.b64encode(bytes(64)).decode()
    return base64.b64encode((f"untrusted comment: signature from tauri secret key\n{packet}\n"
        f"trusted comment: timestamp:1\tfile:{name}\n{global_packet}\n").encode())


class SignatureSafety(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="cassette-signature-tests-")
        self.addCleanup(self.temp.cleanup)
        self.image = Path(self.temp.name) / NAME
        self.image.write_bytes(b"deliberately not an AppImage")
        self.signature = Path(str(self.image) + ".sig")
        self.signature.write_bytes(envelope())

    def test_structure_only(self):
        self.assertEqual(validate_signature(self.signature, VERSION), envelope().decode())

    def test_wrong_filename(self):
        wrong = self.signature.with_name("unversioned.AppImage.sig")
        self.signature.rename(wrong)
        with self.assertRaises(ValueError): validate_signature(wrong, VERSION)

    def test_wrong_signed_filename(self):
        self.signature.write_bytes(envelope("other.AppImage"))
        with self.assertRaises(ValueError): validate_signature(self.signature, VERSION)

    def test_missing_pair(self):
        self.image.unlink()
        with self.assertRaises(ValueError): validate_signature(self.signature, VERSION)

    def test_malformed_empty_large_and_secret_material(self):
        for value in (b"", b"garbage", b"A" * 4097, base64.b64encode(b"-----BEGIN PRIVATE KEY-----"),
                      base64.b64encode(b"untrusted comment: minisign encrypted secret key\nnot-a-key\n")):
            self.signature.write_bytes(value)
            with self.subTest(size=len(value)), self.assertRaises(ValueError):
                validate_signature(self.signature, VERSION)

    def test_symlink_pair(self):
        self.image.unlink()
        other = self.image.with_name("other")
        other.write_bytes(b"other")
        try: self.image.symlink_to(other)
        except OSError: self.skipTest("symlink privilege unavailable")
        with self.assertRaises(ValueError): validate_signature(self.signature, VERSION)

    def test_symlink_signature(self):
        other = self.signature.with_name("detached-envelope")
        self.signature.rename(other)
        try: self.signature.symlink_to(other)
        except OSError: self.skipTest("symlink privilege unavailable")
        with self.assertRaises(ValueError): validate_signature(self.signature, VERSION)

    def test_scanner_recognizes_signature_without_native_version_bytes(self):
        spec = importlib.util.spec_from_file_location("artifact_audit", Path(__file__).with_name("audit-release-artifacts.py"))
        module = importlib.util.module_from_spec(spec)
        sys.modules[spec.name] = module
        spec.loader.exec_module(module)
        self.assertEqual(module.detect_kind(self.signature), "signature")
        root = Path(__file__).resolve().parent.parent
        audit = module.Audit(root, VERSION, root / "LICENSE")
        extraction = Path(self.temp.name) / "inspection"
        extraction.mkdir()
        module.audit_artifact(audit, self.signature, extraction)
        self.assertEqual(audit.errors, set())


if __name__ == "__main__": unittest.main()
