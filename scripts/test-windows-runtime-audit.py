#!/usr/bin/env python3
"""Focused fail-closed checks for the bundled Windows runtime manifest."""

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


module_path = Path(__file__).with_name("audit-release-artifacts.py")
spec = importlib.util.spec_from_file_location("cassette_artifact_audit", module_path)
audit_module = importlib.util.module_from_spec(spec)
import sys
sys.modules[spec.name] = audit_module
spec.loader.exec_module(audit_module)


class WindowsRuntimeAuditTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="cassette-windows-audit-")
        self.root = Path(self.temporary.name)
        self.license = self.root / "LICENSE"
        self.license.write_text("diagnostic test license", encoding="utf-8")
        self.installed = self.root / "app" / "resources"
        self.files = {
            "gstreamer-1.0-0.dll": b"core",
            "lib/gstreamer-1.0/gstplayback.dll": b"plugin",
            "libexec/gstreamer-1.0/gst-plugin-scanner.exe": b"scanner",
            "third-party/gstreamer/NOTICE.txt": b"notice",
        }
        for name, body in self.files.items():
            target = self.installed / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(body)
        manifest = {
            "schemaVersion": 1,
            "gstreamerVersion": audit_module.WINDOWS_GSTREAMER_VERSION,
            "runtimeMsiSha256": audit_module.WINDOWS_GSTREAMER_MSI_SHA256,
            "elementProviders": {name: "gstplayback.dll" for name in audit_module.WINDOWS_REQUIRED_ELEMENTS},
            "files": [
                {"path": name, "sha256": audit_module.sha256_file(self.installed / name)}
                for name in self.files
            ],
        }
        self.manifest = self.installed / "third-party/gstreamer/manifest.json"
        self.manifest.write_text(json.dumps(manifest), encoding="utf-8")

    def tearDown(self):
        self.temporary.cleanup()

    def audit(self):
        result = audit_module.Audit(self.root, audit_module.EXPECTED_VERSION, self.license)
        allowed = audit_module.verify_windows_runtime(result, self.root, "diagnostic")
        return result, allowed

    def test_valid_exact_manifest_is_allowed(self):
        result, allowed = self.audit()
        self.assertFalse(result.errors)
        self.assertIn("app/resources/gstreamer-1.0-0.dll", allowed)

    def test_tampered_runtime_fails(self):
        (self.installed / "gstreamer-1.0-0.dll").write_bytes(b"changed")
        result, _ = self.audit()
        self.assertTrue(any("missing or changed" in error for error in result.errors))

    def test_missing_element_fails(self):
        value = json.loads(self.manifest.read_text(encoding="utf-8"))
        del value["elementProviders"]["avdec_aac"]
        self.manifest.write_text(json.dumps(value), encoding="utf-8")
        result, _ = self.audit()
        self.assertTrue(any("element allowlist" in error for error in result.errors))

    def test_unlisted_gstreamer_dll_still_fails(self):
        result, allowed = self.audit()
        self.assertFalse(result.errors)
        audit_module.audit_filename(
            result, Path("app/resources/lib/gstreamer-1.0/gstnotselected.dll"),
            "diagnostic", True, allowed,
        )
        self.assertTrue(any("bundled external GStreamer" in error for error in result.errors))


if __name__ == "__main__":
    unittest.main()
