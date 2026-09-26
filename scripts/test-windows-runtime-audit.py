#!/usr/bin/env python3
"""Focused fail-closed checks for the bundled Windows runtime manifest."""

import importlib.util
import json
import struct
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
            "vcruntime140.dll": b"signed-microsoft-vc-runtime-fixture",
            "lib/gstreamer-1.0/gstplayback.dll": b"plugin",
            "lib/gstreamer-1.0/gsttypefindfunctions.dll": b"typefind",
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
            "vcRuntime": {
                "source": "Microsoft.VC143.CRT x64",
                "dlls": ["vcruntime140.dll"],
            },
            "elementProviders": {name: "gstplayback.dll" for name in audit_module.WINDOWS_REQUIRED_ELEMENTS},
            "typefindProvider": "gsttypefindfunctions.dll",
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

    def test_missing_typefind_plugin_fails(self):
        value = json.loads(self.manifest.read_text(encoding="utf-8"))
        del value["typefindProvider"]
        self.manifest.write_text(json.dumps(value), encoding="utf-8")
        result, _ = self.audit()
        self.assertTrue(any("typefindfunctions provider" in error for error in result.errors))

    def test_unlisted_hardware_sink_provider_fails(self):
        value = json.loads(self.manifest.read_text(encoding="utf-8"))
        value["elementProviders"]["wasapi2sink"] = "gstmissing.dll"
        self.manifest.write_text(json.dumps(value), encoding="utf-8")
        result, _ = self.audit()
        self.assertTrue(any("provider for wasapi2sink" in error for error in result.errors))

    def test_missing_vc_runtime_fails(self):
        value = json.loads(self.manifest.read_text(encoding="utf-8"))
        del value["vcRuntime"]
        self.manifest.write_text(json.dumps(value), encoding="utf-8")
        result, _ = self.audit()
        self.assertTrue(any("Microsoft VC runtime provenance" in error for error in result.errors))

    def test_unlisted_vc_runtime_fails(self):
        value = json.loads(self.manifest.read_text(encoding="utf-8"))
        value["vcRuntime"]["dlls"] = []
        self.manifest.write_text(json.dumps(value), encoding="utf-8")
        result, _ = self.audit()
        self.assertTrue(any("unlisted Microsoft VC runtime" in error for error in result.errors))

    def test_vc_runtime_path_case_must_match_installed_filename(self):
        value = json.loads(self.manifest.read_text(encoding="utf-8"))
        for entry in value["files"]:
            if entry["path"] == "vcruntime140.dll":
                entry["path"] = "VCRUNTIME140.dll"
        self.manifest.write_text(json.dumps(value), encoding="utf-8")
        result, _ = self.audit()
        self.assertTrue(any("path casing is noncanonical" in error for error in result.errors))

    def test_license_comparison_accepts_only_lf_crlf_difference(self):
        windows_copy = self.root / "windows-license"
        windows_copy.write_bytes(b"GPL text\r\nsecond line\r\n")
        self.license.write_bytes(b"GPL text\nsecond line\n")
        self.assertTrue(audit_module.same_text_ignoring_crlf(windows_copy, self.license))
        windows_copy.write_bytes(b"GPL text\r\nchanged line\r\n")
        self.assertFalse(audit_module.same_text_ignoring_crlf(windows_copy, self.license))

    def test_native_startup_import_must_be_installed_beside_executable(self):
        self.assertTrue({"dnsapi.dll", "iphlpapi.dll"}.issubset(
            audit_module.WINDOWS_SYSTEM_DLLS,
        ))
        application = self.root / "cassette.exe"
        application.write_bytes(self.synthetic_pe_importing("gio-2.0-0.dll"))
        result = audit_module.Audit(self.root, audit_module.EXPECTED_VERSION, self.license)
        audit_module.verify_windows_binary_imports(
            result, self.root, application, set(), "synthetic installer",
        )
        self.assertTrue(any("gio-2.0-0.dll, absent beside cassette.exe" in error
                            for error in result.errors))

        (self.root / "gio-2.0-0.dll").write_bytes(self.synthetic_pe_importing("KERNEL32.dll"))
        result = audit_module.Audit(self.root, audit_module.EXPECTED_VERSION, self.license)
        audit_module.verify_windows_binary_imports(
            result, self.root, application, {"gio-2.0-0.dll"}, "synthetic installer",
        )
        self.assertFalse(result.errors)

    def test_wasapi_system_imports_are_exactly_allowlisted(self):
        self.assertTrue({"mfplat.dll", "mmdevapi.dll"}.issubset(
            audit_module.WINDOWS_SYSTEM_DLLS,
        ))
        self.assertNotIn("mfreadwrite.dll", audit_module.WINDOWS_SYSTEM_DLLS)

    @staticmethod
    def synthetic_pe_importing(name):
        data = bytearray(2048)
        data[:2] = b"MZ"
        struct.pack_into("<I", data, 60, 0x80)
        data[0x80:0x84] = b"PE\0\0"
        struct.pack_into("<HH", data, 0x84, 0x8664, 1)
        struct.pack_into("<H", data, 0x84 + 16, 240)
        optional = 0x84 + 20
        struct.pack_into("<H", data, optional, 0x20B)
        struct.pack_into("<I", data, optional + 108, 16)
        struct.pack_into("<II", data, optional + 112 + 8, 0x1000, 40)
        section = optional + 240
        data[section:section + 8] = b".rdata\0\0"
        struct.pack_into("<IIII", data, section + 8, 0x600, 0x1000, 0x600, 0x200)
        struct.pack_into("<IIIII", data, 0x200, 0, 0, 0, 0x1100, 0)
        encoded = name.encode("ascii") + b"\0"
        data[0x300:0x300 + len(encoded)] = encoded
        return bytes(data)

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
