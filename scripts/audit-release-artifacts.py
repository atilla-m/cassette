#!/usr/bin/env python3
"""Fail closed on unsafe or inconsistent Cassette release artifacts.

The scanner uses only Python's standard library plus native packaging tools that
are already required by the platform workflow. It intentionally permits the
exact Tauri development endpoints in native binaries only: Tauri can retain
those inert configuration strings even when production assets are embedded.
The same URLs remain fatal in built frontend files or the production CSP.
"""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import os
import re
import shutil
import struct
import subprocess
import sys
import tarfile
import tempfile
from dataclasses import dataclass, field
from pathlib import Path
from update_signature import validate_signature


EXPECTED_VERSION = "0.1.0-beta.3"
EXPECTED_LICENSE = "GPL-3.0-or-later"
WINDOWS_GSTREAMER_VERSION = "1.26.11"
WINDOWS_GSTREAMER_MSI_SHA256 = "31cbc21fa0950b5c1e79c80959b2799805cb05a7a35953a13a9f790776137605"
WINDOWS_REQUIRED_ELEMENTS = {
    "playbin", "uridecodebin", "decodebin3", "filesrc", "typefind",
    "audioconvert", "audioresample", "autoaudiosink", "id3demux",
    "flacdec", "oggdemux", "vorbisdec", "opusdec", "wavparse",
    "qtdemux", "avdec_mp3", "avdec_aac",
}
# Only operating-system DLLs may be absent from Cassette's installed folder.
# Keep this deliberately narrow: new imports require an explicit review.
WINDOWS_SYSTEM_DLLS = {
    "advapi32.dll", "bcrypt.dll", "bcryptprimitives.dll", "comctl32.dll",
    "crypt32.dll", "dnsapi.dll", "dwmapi.dll", "gdi32.dll", "iphlpapi.dll",
    "kernel32.dll", "ntdll.dll", "ole32.dll", "oleaut32.dll", "shell32.dll",
    "shlwapi.dll", "user32.dll",
    "ws2_32.dll",
}
NATIVE_TAURI_DEV_URL_EXCEPTIONS = {
    b"http://localhost:1420",
    b"ws://localhost:1420",
}
FRONTEND_SUFFIXES = {".css", ".html", ".js", ".json", ".map", ".mjs"}
DATABASE_SUFFIXES = {".db", ".sqlite", ".sqlite3", ".db-shm", ".db-wal"}
MEDIA_SUFFIXES = {
    ".aac", ".aiff", ".avi", ".flac", ".m4a", ".m4v", ".mkv",
    ".mov", ".mp3", ".mp4", ".ogg", ".opus", ".vob", ".wav", ".webm",
}
PRIVATE_KEY_SUFFIXES = {".key", ".p12", ".pfx"}

DEV_URL_PATTERN = re.compile(
    rb"(?:https?|wss?)://(?:localhost|127\.0\.0\.1|\[::1\])(?::[0-9]+)?",
    re.IGNORECASE,
)
SECRET_PATTERNS = {
    "PEM private key": re.compile(rb"-----BEGIN (?:[A-Z0-9 ]+ )?PRIVATE KEY-----"),
    "AWS access key": re.compile(rb"AKIA[0-9A-Z]{16}"),
    "GitHub token": re.compile(rb"(?:gh[pousr]_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,})"),
    "Slack token": re.compile(rb"xox[baprs]-[A-Za-z0-9-]{20,}"),
}


@dataclass
class Audit:
    workspace: Path
    expected_version: str
    license_path: Path
    errors: set[str] = field(default_factory=set)
    warnings: set[str] = field(default_factory=set)
    scanned_files: int = 0
    native_dev_url_exceptions: int = 0

    @property
    def license_hash(self) -> str:
        return sha256_file(self.license_path)

    def fail(self, message: str) -> None:
        self.errors.add(message)

    def warn(self, message: str) -> None:
        self.warnings.add(message)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def same_text_ignoring_crlf(first: Path, second: Path) -> bool:
    """Compare tracked text after Windows checkout's LF-to-CRLF conversion."""
    return first.read_bytes().replace(b"\r\n", b"\n") == second.read_bytes().replace(b"\r\n", b"\n")


def command_path(name: str) -> str | None:
    return shutil.which(name)


def run_checked(args: list[str], *, cwd: Path | None = None, stdin=None) -> subprocess.CompletedProcess:
    result = subprocess.run(
        args,
        cwd=cwd,
        stdin=stdin,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0:
        detail = result.stderr.decode("utf-8", "replace").strip()
        raise RuntimeError(f"{' '.join(args)} failed with exit code {result.returncode}: {detail}")
    return result


def safe_tar_extract(archive: tarfile.TarFile, destination: Path) -> None:
    destination_root = destination.resolve()
    for member in archive.getmembers():
        resolved = (destination / member.name).resolve()
        if resolved != destination_root and destination_root not in resolved.parents:
            raise RuntimeError(f"Archive path escapes extraction root: {member.name}")
    try:
        archive.extractall(destination, filter="data")
    except TypeError:  # Python before 3.12.
        archive.extractall(destination)


def read_ar_members(path: Path) -> dict[str, bytes]:
    data = path.read_bytes()
    if not data.startswith(b"!<arch>\n"):
        raise RuntimeError(f"Not an ar archive: {path}")

    members: dict[str, bytes] = {}
    offset = 8
    while offset < len(data):
        header = data[offset:offset + 60]
        if len(header) != 60 or header[58:60] != b"`\n":
            raise RuntimeError(f"Invalid ar member header in {path}")
        name = header[:16].decode("ascii", "replace").strip().rstrip("/")
        size = int(header[48:58].decode("ascii").strip())
        start = offset + 60
        members[name] = data[start:start + size]
        offset = start + size + (size % 2)
    return members


def extract_deb(path: Path, destination: Path) -> tuple[Path, str]:
    members = read_ar_members(path)
    data_name = next((name for name in members if name.startswith("data.tar")), None)
    control_name = next((name for name in members if name.startswith("control.tar")), None)
    if not data_name or not control_name:
        raise RuntimeError(f"DEB is missing data/control archives: {path}")

    with tarfile.open(fileobj=io.BytesIO(members[data_name]), mode="r:*") as archive:
        safe_tar_extract(archive, destination)

    with tarfile.open(fileobj=io.BytesIO(members[control_name]), mode="r:*") as archive:
        control_member = next(
            (member for member in archive.getmembers() if Path(member.name).name == "control"),
            None,
        )
        if control_member is None:
            raise RuntimeError(f"DEB control archive has no control file: {path}")
        extracted = archive.extractfile(control_member)
        if extracted is None:
            raise RuntimeError(f"Could not read DEB control metadata: {path}")
        control = extracted.read().decode("utf-8", "replace")
    return destination, control


def extract_rpm(path: Path, destination: Path) -> Path:
    bsdtar = command_path("bsdtar")
    if bsdtar:
        listing = run_checked([bsdtar, "-tf", str(path)]).stdout.decode("utf-8", "replace")
        for member in listing.splitlines():
            member_path = Path(member)
            if member_path.is_absolute() or ".." in member_path.parts:
                raise RuntimeError(f"RPM path escapes extraction root: {member}")
        run_checked([
            bsdtar,
            "-xf", str(path),
            "-C", str(destination),
            "--no-same-owner",
        ])
        return destination

    rpm2cpio = command_path("rpm2cpio")
    cpio = command_path("cpio")
    if not rpm2cpio or not cpio:
        raise RuntimeError("RPM inspection requires bsdtar or both rpm2cpio and cpio")

    # Do not connect these processes with a live pipe. cpio is allowed to stop
    # reading at the archive trailer, which can give rpm2cpio a timing-dependent
    # SIGPIPE while it flushes trailing padding even though extraction succeeded.
    with tempfile.TemporaryFile() as payload:
        producer = subprocess.run(
            [rpm2cpio, str(path)],
            stdout=payload,
            stderr=subprocess.PIPE,
            check=False,
        )
        if producer.returncode != 0:
            detail = producer.stderr.decode("utf-8", "replace").strip() or "no diagnostic output"
            raise RuntimeError(
                f"Could not convert RPM {path} (rpm2cpio exit {producer.returncode}): {detail}"
            )

        payload.seek(0)
        consumer = subprocess.run(
            [cpio, "-idm", "--quiet", "--no-absolute-filenames"],
            cwd=destination,
            stdin=payload,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        if consumer.returncode != 0:
            detail = consumer.stderr.decode("utf-8", "replace").strip() or "no diagnostic output"
            raise RuntimeError(
                f"Could not extract RPM {path} (cpio exit {consumer.returncode}): {detail}"
            )
    return destination


def extract_appimage(path: Path, destination: Path) -> Path:
    result = run_checked([str(path.resolve()), "--appimage-extract"], cwd=destination)
    root = destination / "squashfs-root"
    if not root.is_dir():
        detail = result.stdout.decode("utf-8", "replace").strip()
        raise RuntimeError(f"AppImage extraction produced no squashfs-root: {detail}")
    return root


def extract_nsis(path: Path, destination: Path) -> Path:
    seven_zip = command_path("7z") or command_path("7z.exe")
    if not seven_zip:
        raise RuntimeError("NSIS inspection requires 7-Zip")
    run_checked([seven_zip, "x", "-y", f"-o{destination}", str(path.resolve())])

    nested_archives = sorted(destination.rglob("*.7z"))
    for index, archive in enumerate(nested_archives):
        nested_destination = destination / f"nested-{index}"
        nested_destination.mkdir()
        run_checked([seven_zip, "x", "-y", f"-o{nested_destination}", str(archive)])
    return destination


def extract_msi(path: Path, destination: Path) -> Path | None:
    lessmsi = command_path("lessmsi") or command_path("lessmsi.exe")
    if lessmsi:
        run_checked([lessmsi, "x", str(path.resolve()), str(destination)])
        return destination

    msiexec = command_path("msiexec.exe") or command_path("msiexec")
    if msiexec and os.name == "nt":
        log_path = destination.parent / "msi-extract.log"
        run_checked([
            msiexec,
            "/a", str(path.resolve()),
            "/qn",
            f"TARGETDIR={destination}",
            "/L*v", str(log_path),
        ])
        return destination
    return None


def detect_kind(path: Path) -> str:
    lower_name = path.name.lower()
    if path.is_dir():
        return "appdir" if lower_name.endswith(".appdir") else "directory"
    if lower_name.endswith(".sig"):
        return "signature"
    if lower_name.endswith(".appimage"):
        return "appimage"
    if lower_name.endswith(".deb"):
        return "deb"
    if lower_name.endswith(".rpm"):
        return "rpm"
    if lower_name.endswith(".msi"):
        return "msi"
    if lower_name.endswith("-setup.exe"):
        return "nsis"
    return "native"


def file_is_x86_64_elf(path: Path) -> bool:
    with path.open("rb") as handle:
        header = handle.read(20)
    if len(header) < 20 or header[:4] != b"\x7fELF" or header[4] != 2:
        return False
    byte_order = "<" if header[5] == 1 else ">"
    return struct.unpack(f"{byte_order}H", header[18:20])[0] == 62


def file_is_x86_64_pe(path: Path) -> bool:
    with path.open("rb") as handle:
        dos = handle.read(64)
        if len(dos) < 64 or dos[:2] != b"MZ":
            return False
        pe_offset = struct.unpack("<I", dos[60:64])[0]
        handle.seek(pe_offset)
        pe_header = handle.read(6)
    return pe_header[:4] == b"PE\0\0" and struct.unpack("<H", pe_header[4:6])[0] == 0x8664


def pe_resource_types(path: Path) -> set[int]:
    """Return numeric top-level PE resource types without external tooling."""
    data = path.read_bytes()
    if len(data) < 64 or data[:2] != b"MZ":
        return set()
    pe_offset = struct.unpack_from("<I", data, 60)[0]
    if data[pe_offset:pe_offset + 4] != b"PE\0\0":
        return set()

    coff_offset = pe_offset + 4
    section_count = struct.unpack_from("<H", data, coff_offset + 2)[0]
    optional_size = struct.unpack_from("<H", data, coff_offset + 16)[0]
    optional_offset = coff_offset + 20
    magic = struct.unpack_from("<H", data, optional_offset)[0]
    data_directory_offset = optional_offset + (112 if magic == 0x20B else 96 if magic == 0x10B else 0)
    if data_directory_offset == optional_offset:
        return set()
    resource_rva, resource_size = struct.unpack_from("<II", data, data_directory_offset + (2 * 8))
    if not resource_rva or not resource_size:
        return set()

    section_offset = optional_offset + optional_size
    resource_offset = None
    for index in range(section_count):
        header_offset = section_offset + index * 40
        virtual_size, virtual_address, raw_size, raw_offset = struct.unpack_from(
            "<IIII", data, header_offset + 8,
        )
        if virtual_address <= resource_rva < virtual_address + max(virtual_size, raw_size):
            resource_offset = raw_offset + (resource_rva - virtual_address)
            break
    if resource_offset is None or resource_offset + 16 > len(data):
        return set()

    named_count, id_count = struct.unpack_from("<HH", data, resource_offset + 12)
    types: set[int] = set()
    for index in range(named_count + id_count):
        entry_offset = resource_offset + 16 + index * 8
        if entry_offset + 8 > len(data):
            break
        identifier = struct.unpack_from("<I", data, entry_offset)[0]
        if not identifier & 0x80000000:
            types.add(identifier & 0xFFFF)
    return types


def pe_imported_dlls(path: Path) -> set[str]:
    """Read PE import names without depending on the build host's DLL search path."""
    data = path.read_bytes()
    if len(data) < 64 or data[:2] != b"MZ":
        raise ValueError(f"not a PE file: {path}")
    pe_offset = struct.unpack_from("<I", data, 60)[0]
    if pe_offset + 24 > len(data) or data[pe_offset:pe_offset + 4] != b"PE\0\0":
        raise ValueError(f"invalid PE header: {path}")
    coff = pe_offset + 4
    section_count = struct.unpack_from("<H", data, coff + 2)[0]
    optional_size = struct.unpack_from("<H", data, coff + 16)[0]
    optional = coff + 20
    if optional + optional_size > len(data):
        raise ValueError(f"truncated PE optional header: {path}")
    magic = struct.unpack_from("<H", data, optional)[0]
    if magic not in (0x10B, 0x20B):
        raise ValueError(f"unsupported PE optional header: {path}")
    directory = optional + (112 if magic == 0x20B else 96)
    directory_count = struct.unpack_from("<I", data, directory - 4)[0]
    if directory_count < 2 or directory + 16 > optional + optional_size:
        return set()
    import_rva, _ = struct.unpack_from("<II", data, directory + 8)
    if not import_rva:
        return set()
    sections = []
    section_start = optional + optional_size
    for index in range(section_count):
        offset = section_start + index * 40
        if offset + 40 > len(data):
            raise ValueError(f"truncated PE section table: {path}")
        virtual_size, virtual_address, raw_size, raw_offset = struct.unpack_from(
            "<IIII", data, offset + 8,
        )
        sections.append((virtual_address, max(virtual_size, raw_size), raw_size, raw_offset))

    def rva_to_offset(rva: int) -> int:
        for address, extent, raw_size, offset in sections:
            displacement = rva - address
            if 0 <= displacement < extent and displacement < raw_size:
                resolved = offset + displacement
                if resolved < len(data):
                    return resolved
        raise ValueError(f"unmapped PE import RVA in {path}")

    imports: set[str] = set()
    descriptor = rva_to_offset(import_rva)
    for _ in range(4096):
        if descriptor + 20 > len(data):
            raise ValueError(f"truncated PE import table: {path}")
        values = struct.unpack_from("<IIIII", data, descriptor)
        if values == (0, 0, 0, 0, 0):
            return imports
        name_offset = rva_to_offset(values[3])
        name_end = data.find(b"\0", name_offset, min(name_offset + 256, len(data)))
        if name_end < 0:
            raise ValueError(f"unterminated PE import name: {path}")
        name = data[name_offset:name_end].decode("ascii", "strict").lower()
        if not re.fullmatch(r"[a-z0-9_.+-]+\.dll", name):
            raise ValueError(f"invalid PE import name {name!r}: {path}")
        imports.add(name)
        descriptor += 20
    raise ValueError(f"oversized PE import table: {path}")


def parse_control_fields(control: str) -> dict[str, str]:
    fields: dict[str, str] = {}
    current: str | None = None
    for line in control.splitlines():
        if line.startswith((" ", "\t")) and current:
            fields[current] += " " + line.strip()
        elif ":" in line:
            current, value = line.split(":", 1)
            fields[current] = value.strip()
    return fields


def require_file_hash(audit: Audit, root: Path, relative: str, artifact_label: str) -> None:
    candidate = root / relative
    if not candidate.is_file():
        audit.fail(f"{artifact_label}: missing required license payload {relative}")
        return
    actual = sha256_file(candidate)
    if actual != audit.license_hash:
        audit.fail(f"{artifact_label}: license hash mismatch at {relative}")


def require_linux_payload(audit: Audit, root: Path, artifact_label: str) -> None:
    required = (
        "usr/bin/cassette",
        "usr/lib/Cassette/LICENSE",
        "usr/share/applications/Cassette.desktop",
        "usr/share/icons/hicolor/32x32/apps/cassette.png",
        "usr/share/icons/hicolor/128x128/apps/cassette.png",
        "usr/share/icons/hicolor/256x256@2/apps/cassette.png",
    )
    for relative in required:
        if not (root / relative).is_file():
            audit.fail(f"{artifact_label}: missing expected payload file {relative}")
    executable = root / "usr/bin/cassette"
    if executable.is_file() and not file_is_x86_64_elf(executable):
        audit.fail(f"{artifact_label}: Cassette executable is not x86_64 ELF")


def audit_configuration(audit: Audit) -> None:
    package = json.loads((audit.workspace / "package.json").read_text(encoding="utf-8"))
    tauri = json.loads((audit.workspace / "src-tauri/tauri.conf.json").read_text(encoding="utf-8"))
    cargo_text = (audit.workspace / "src-tauri/Cargo.toml").read_text(encoding="utf-8")

    if package.get("version") != audit.expected_version:
        audit.fail("package.json version does not match the expected release version")
    if package.get("license") != EXPECTED_LICENSE:
        audit.fail("package.json license is not GPL-3.0-or-later")
    if tauri.get("version") != audit.expected_version:
        audit.fail("tauri.conf.json version does not match the expected release version")

    bundle = tauri.get("bundle", {})
    if bundle.get("license") != EXPECTED_LICENSE or bundle.get("licenseFile") != "../LICENSE":
        audit.fail("Tauri license metadata is incomplete or inconsistent")
    if bundle.get("resources", {}).get("../LICENSE") != "LICENSE":
        audit.fail("Tauri global LICENSE resource mapping is missing")
    if f'version = "{audit.expected_version}"' not in cargo_text:
        audit.fail("Cargo.toml version does not match the expected release version")
    if f'license = "{EXPECTED_LICENSE}"' not in cargo_text:
        audit.fail("Cargo.toml license is not GPL-3.0-or-later")

    icon_paths = bundle.get("icon", [])
    if not icon_paths:
        audit.fail("Tauri has no configured application icons")
    for icon in icon_paths:
        if not (audit.workspace / "src-tauri" / icon).is_file():
            audit.fail(f"Configured Tauri icon is missing: {icon}")

    production_csp = tauri.get("app", {}).get("security", {}).get("csp", "") or ""
    if DEV_URL_PATTERN.search(production_csp.encode("utf-8")):
        audit.fail("Production CSP contains a development frontend URL")


def audit_filename(
    audit: Audit, relative: Path, artifact_label: str, windows_bundle: bool,
    allowed_windows_runtime: set[str] | None = None,
) -> None:
    normalized = relative.as_posix().lower()
    name = relative.name.lower()
    suffix = relative.suffix.lower()

    if suffix in DATABASE_SUFFIXES or any(name.endswith(value) for value in DATABASE_SUFFIXES):
        audit.fail(f"{artifact_label}: packaged database file {relative.as_posix()}")
    if suffix in MEDIA_SUFFIXES:
        audit.fail(f"{artifact_label}: packaged user-media-like file {relative.as_posix()}")
    if suffix == ".lrc" or any(
        segment in normalized
        for segment in ("cover-art/", "video-thumbnails/", "cached-lyrics/", "lyrics-cache/")
    ):
        audit.fail(f"{artifact_label}: packaged cache/media path {relative.as_posix()}")
    if suffix in {".jpg", ".jpeg", ".png", ".webp"} and any(
        marker in name for marker in ("cover", "artwork", "thumbnail", "cached-lyric")
    ):
        audit.fail(f"{artifact_label}: packaged cover/cache image {relative.as_posix()}")
    private_pem = suffix == ".pem" and any(marker in name for marker in ("private", "key"))
    if suffix in PRIVATE_KEY_SUFFIXES or private_pem or name == ".env":
        audit.fail(f"{artifact_label}: packaged credential-like file {relative.as_posix()}")

    if windows_bundle and normalized not in (allowed_windows_runtime or set()):
        if "gstreamer" in normalized and suffix in {".dll", ".exe", ".cache"}:
            audit.fail(f"{artifact_label}: bundled external GStreamer runtime file {relative.as_posix()}")
        if suffix == ".dll" and (name.startswith("gst") or name.startswith("libgst")):
            audit.fail(f"{artifact_label}: bundled external GStreamer DLL {relative.as_posix()}")


def content_patterns(audit: Audit) -> dict[str, re.Pattern[bytes]]:
    patterns: dict[str, re.Pattern[bytes]] = {}
    developer_home = Path.home().resolve()
    path_groups = {
        "developer home directory": {
            str(developer_home),
            developer_home.as_posix(),
            str(developer_home).replace("\\", "/"),
            str(developer_home).replace("/", "\\"),
        },
        "repository/workspace absolute path": {
            str(audit.workspace.resolve()),
            audit.workspace.resolve().as_posix(),
            str(audit.workspace.resolve()).replace("\\", "/"),
            str(audit.workspace.resolve()).replace("/", "\\"),
        },
    }
    for label, variants in path_groups.items():
        for index, value in enumerate(sorted(variants)):
            separator = b"[\\\\/]" if not value.endswith(("/", "\\")) else b""
            patterns[f"{label} #{index + 1}"] = re.compile(re.escape(value.encode()) + separator)
    return patterns


def scan_file(
    audit: Audit,
    path: Path,
    relative: Path,
    artifact_label: str,
    *,
    frontend: bool,
    native: bool,
    scan_secrets: bool,
    patterns: dict[str, re.Pattern[bytes]],
) -> None:
    audit.scanned_files += 1
    found: set[str] = set()
    allowed_native_urls: set[bytes] = set()
    tail = b""

    try:
        handle = path.open("rb")
    except OSError as error:
        audit.fail(f"{artifact_label}: could not read {relative.as_posix()}: {error}")
        return

    with handle:
        for raw_chunk in iter(lambda: handle.read(1024 * 1024), b""):
            chunk = tail + raw_chunk
            for label, pattern in patterns.items():
                if label not in found and pattern.search(chunk):
                    found.add(label)
            if scan_secrets:
                for label, pattern in SECRET_PATTERNS.items():
                    if label not in found and pattern.search(chunk):
                        found.add(label)

            for match in DEV_URL_PATTERN.finditer(chunk):
                url = match.group(0)
                if native and not frontend and url in NATIVE_TAURI_DEV_URL_EXCEPTIONS:
                    allowed_native_urls.add(url)
                else:
                    found.add("unexpected development frontend URL")
            tail = chunk[-1024:]

    for label in found:
        audit.fail(f"{artifact_label}: {label} in {relative.as_posix()}")
    if allowed_native_urls:
        audit.native_dev_url_exceptions += len(allowed_native_urls)


def scan_tree(
    audit: Audit, root: Path, artifact_label: str, *, windows_bundle: bool = False,
    allowed_windows_runtime: set[str] | None = None,
) -> None:
    patterns = content_patterns(audit)
    for path in sorted(candidate for candidate in root.rglob("*") if candidate.is_file()):
        relative = path.relative_to(root)
        audit_filename(audit, relative, artifact_label, windows_bundle, allowed_windows_runtime)
        suffix = path.suffix.lower()
        frontend = suffix in FRONTEND_SUFFIXES
        native = path.name.lower() in {"cassette", "cassette.exe"}
        normalized = relative.as_posix().lower()
        app_owned = (
            frontend
            or path.name.lower() in {"cassette", "cassette.exe"}
            or normalized.startswith("usr/lib/cassette/")
        )
        scan_file(
            audit,
            path,
            relative,
            artifact_label,
            frontend=frontend,
            native=native,
            scan_secrets=app_owned,
            patterns=patterns,
        )


def scan_frontend(audit: Audit, root: Path) -> None:
    if not root.is_dir():
        audit.fail(f"Built frontend directory does not exist: {root}")
        return
    patterns = content_patterns(audit)
    version_found = False
    for path in sorted(candidate for candidate in root.rglob("*") if candidate.is_file()):
        relative = path.relative_to(root)
        audit_filename(audit, relative, "frontend", False)
        data = path.read_bytes()
        if audit.expected_version.encode() in data:
            version_found = True
        scan_file(
            audit,
            path,
            relative,
            "frontend",
            frontend=True,
            native=False,
            scan_secrets=True,
            patterns=patterns,
        )
    if not version_found:
        audit.fail("Built frontend does not contain the expected release version")


def verify_deb(audit: Audit, path: Path, root: Path, control: str) -> None:
    label = path.name
    fields = parse_control_fields(control)
    if fields.get("Version") != audit.expected_version:
        audit.fail(f"{label}: DEB version is {fields.get('Version')!r}")
    if fields.get("Architecture") != "amd64":
        audit.fail(f"{label}: DEB architecture is {fields.get('Architecture')!r}")

    dependencies = {value.strip() for value in fields.get("Depends", "").split(",")}
    expected = {
        "libgstreamer1.0-0", "gstreamer1.0-plugins-base", "gstreamer1.0-plugins-good",
        "gstreamer1.0-plugins-bad", "gstreamer1.0-plugins-ugly", "gstreamer1.0-libav",
        "libwebkit2gtk-4.1-0", "libgtk-3-0",
    }
    missing = sorted(expected - dependencies)
    if missing:
        audit.fail(f"{label}: missing DEB dependencies: {', '.join(missing)}")
    if fields.get("Recommends") != "libnotify-bin":
        audit.fail(f"{label}: DEB Recommends is not exactly libnotify-bin")

    require_linux_payload(audit, root, label)
    require_file_hash(audit, root, "usr/lib/Cassette/LICENSE", label)
    require_file_hash(audit, root, "usr/share/doc/cassette/LICENSE", label)


def verify_rpm(audit: Audit, path: Path, root: Path) -> None:
    label = path.name
    rpm = command_path("rpm")
    if not rpm:
        audit.fail(f"{label}: RPM metadata inspection requires rpm")
        return
    query = run_checked([
        rpm, "-qp", "--qf", "%{VERSION}\n%{RELEASE}\n%{ARCH}\n%{LICENSE}\n", str(path),
    ]).stdout.decode("utf-8", "replace").splitlines()
    if query[:4] != [audit.expected_version, "1", "x86_64", EXPECTED_LICENSE]:
        audit.fail(f"{label}: unexpected RPM version/release/architecture/license metadata: {query[:4]}")

    requirements = set(
        run_checked([rpm, "-qpR", str(path)]).stdout.decode("utf-8", "replace").splitlines()
    )
    expected = {
        "webkit2gtk4.1", "gtk3", "gstreamer1", "gstreamer1-plugins-base",
        "gstreamer1-plugins-good", "gstreamer1-plugins-bad-free",
        "gstreamer1-plugins-ugly-free", "gstreamer1-libav",
    }
    missing = sorted(expected - requirements)
    if missing:
        audit.fail(f"{label}: missing RPM requirements: {', '.join(missing)}")
    recommends = run_checked([rpm, "-qp", "--recommends", str(path)]).stdout.decode().splitlines()
    if recommends != ["libnotify"]:
        audit.fail(f"{label}: RPM Recommends is not exactly libnotify")

    require_linux_payload(audit, root, label)
    require_file_hash(audit, root, "usr/lib/Cassette/LICENSE", label)
    require_file_hash(audit, root, "usr/share/licenses/cassette/LICENSE", label)


def verify_appdir(audit: Audit, root: Path, label: str) -> None:
    require_linux_payload(audit, root, label)
    require_file_hash(audit, root, "usr/lib/Cassette/LICENSE", label)
    verify_appdir_host_libraries(audit, root, label)


def verify_appdir_host_libraries(audit: Audit, root: Path, label: str) -> None:
    # Inspect names without dereferencing: dangling SONAME symlinks must fail too.
    # This is independent of appimagetool's exclusion file and runs on the
    # extracted final AppImage, after the packaging-tool exclusion is applied.
    for path in root.rglob("libwayland-client.so*"):
        audit.fail(
            f"{label}: bundled host graphics dependency {path.relative_to(root)}; "
            "libwayland-client must resolve from the host for Mesa compatibility"
        )


def verify_windows_runtime(audit: Audit, root: Path, label: str) -> set[str]:
    marker = "third-party/gstreamer/manifest.json"
    manifests = [
        path for path in root.rglob("manifest.json")
        if path.relative_to(root).as_posix().lower().endswith(marker)
    ]
    if len(manifests) != 1:
        audit.fail(f"{label}: expected exactly one bundled GStreamer manifest, found {len(manifests)}")
        return set()
    manifest_path = manifests[0]
    relative_marker = manifest_path.relative_to(root).as_posix()
    installation_prefix = relative_marker[:-len(marker)]
    allowed = {relative_marker.lower()}
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8-sig"))
    except (OSError, ValueError) as error:
        audit.fail(f"{label}: invalid GStreamer manifest: {error}")
        return allowed
    if (manifest.get("schemaVersion") != 1
            or manifest.get("gstreamerVersion") != WINDOWS_GSTREAMER_VERSION
            or manifest.get("runtimeMsiSha256") != WINDOWS_GSTREAMER_MSI_SHA256):
        audit.fail(f"{label}: bundled GStreamer provenance is incorrect")
    if set(manifest.get("elementProviders", {})) != WINDOWS_REQUIRED_ELEMENTS:
        audit.fail(f"{label}: GStreamer element allowlist is incomplete")
    vc_runtime = manifest.get("vcRuntime")
    vc_dlls: set[str] = set()
    if (not isinstance(vc_runtime, dict)
            or vc_runtime.get("source") != "Microsoft.VC143.CRT x64"
            or not isinstance(vc_runtime.get("dlls"), list)):
        audit.fail(f"{label}: Microsoft VC runtime provenance is missing")
    else:
        names = vc_runtime["dlls"]
        permitted_vc = re.compile(
            r"(?:vcruntime140(?:_1)?|msvcp140(?:_1|_2|_atomic_wait|_codecvt_ids)?|concrt140)\.dll",
            re.IGNORECASE,
        )
        if (not all(isinstance(name, str) and name == name.lower()
                    and permitted_vc.fullmatch(name) for name in names)
                or len({name.lower() for name in names}) != len(names)):
            audit.fail(f"{label}: Microsoft VC runtime file list is unsafe")
        else:
            vc_dlls = {name.lower() for name in names}
            if "vcruntime140.dll" not in vc_dlls:
                audit.fail(f"{label}: required Microsoft VC runtime is missing")

    files = manifest.get("files")
    if not isinstance(files, list) or not files:
        audit.fail(f"{label}: GStreamer manifest has no files")
        return allowed
    seen: set[str] = set()
    for entry in files:
        if not isinstance(entry, dict):
            audit.fail(f"{label}: invalid GStreamer file entry")
            continue
        name, digest = entry.get("path"), entry.get("sha256")
        if (not isinstance(name, str) or not name or name.startswith("/")
                or "\\" in name or ".." in name.split("/")
                or not isinstance(digest, str) or not re.fullmatch(r"[0-9a-f]{64}", digest)):
            audit.fail(f"{label}: unsafe GStreamer file entry")
            continue
        normalized = name.lower()
        if normalized in vc_dlls and name != normalized:
            audit.fail(f"{label}: Microsoft VC runtime path casing is noncanonical: {name}")
        if normalized in seen:
            audit.fail(f"{label}: duplicate GStreamer file {name}")
            continue
        seen.add(normalized)
        permitted = (
            ("/" not in name and normalized.endswith(".dll"))
            or (normalized.startswith("lib/gstreamer-1.0/") and normalized.endswith(".dll"))
            or normalized == "libexec/gstreamer-1.0/gst-plugin-scanner.exe"
            or (normalized.startswith("third-party/gstreamer/")
                and Path(name).suffix.lower() in {".txt", ".md", "", ".license"})
        )
        if not permitted:
            audit.fail(f"{label}: disallowed bundled GStreamer path {name}")
            continue
        relative = installation_prefix + name
        candidate = root / relative
        if not candidate.is_file() or sha256_file(candidate) != digest:
            audit.fail(f"{label}: bundled GStreamer file missing or changed: {name}")
        allowed.add(relative.lower())
    if "gstreamer-1.0-0.dll" not in seen:
        audit.fail(f"{label}: bundled GStreamer core DLL is missing")
    if not vc_dlls.issubset(seen):
        audit.fail(f"{label}: listed Microsoft VC runtime files are missing")
    if any(name in seen and name not in vc_dlls for name in (
            "vcruntime140.dll", "vcruntime140_1.dll", "msvcp140.dll",
            "msvcp140_1.dll", "msvcp140_2.dll", "msvcp140_atomic_wait.dll",
            "msvcp140_codecvt_ids.dll", "concrt140.dll")):
        audit.fail(f"{label}: an unlisted Microsoft VC runtime file was bundled")
    if "third-party/gstreamer/notice.txt" not in seen:
        audit.fail(f"{label}: bundled GStreamer notice is missing")
    return allowed


def verify_windows_binary_imports(
    audit: Audit, root: Path, executable: Path, allowed_runtime: set[str], label: str,
) -> None:
    """Ensure loader-time imports resolve within the installed app or Windows."""
    application_dir = executable.parent
    local_dlls = {
        path.name.lower(): path
        for path in application_dir.iterdir()
        if path.is_file() and path.suffix.lower() == ".dll"
    }
    binaries = [executable]
    packaged = {
        path.relative_to(root).as_posix().lower(): path
        for path in root.rglob("*") if path.is_file()
    }
    for relative in sorted(allowed_runtime):
        candidate = packaged.get(relative)
        if candidate and candidate.suffix.lower() in {".dll", ".exe"}:
            binaries.append(candidate)
    for binary in binaries:
        try:
            imports = pe_imported_dlls(binary)
        except (OSError, UnicodeError, ValueError) as error:
            audit.fail(f"{label}: cannot inspect PE imports of {binary.name}: {error}")
            continue
        for name in imports:
            if name.startswith(("api-ms-win-", "ext-ms-win-")) or name in WINDOWS_SYSTEM_DLLS:
                continue
            dependency = local_dlls.get(name)
            if dependency is None:
                audit.fail(
                    f"{label}: {binary.name} imports {name}, absent beside cassette.exe "
                    "before startup"
                )
            elif dependency.relative_to(root).as_posix().lower() not in allowed_runtime:
                audit.fail(f"{label}: {binary.name} imports unlisted DLL {name}")


def verify_windows_payload(audit: Audit, root: Path, label: str) -> set[str]:
    executables = [path for path in root.rglob("*.exe") if path.name.lower() == "cassette.exe"]
    if not executables:
        audit.fail(f"{label}: extracted installer contains no Cassette executable")
    elif not any(file_is_x86_64_pe(path) for path in executables):
        audit.fail(f"{label}: extracted Cassette executable is not x86_64 PE")
    elif not any({3, 14}.issubset(pe_resource_types(path)) for path in executables):
        audit.fail(f"{label}: extracted Cassette executable has no complete Windows icon resource")

    licenses = [path for path in root.rglob("*") if path.is_file() and path.name.upper() == "LICENSE"]
    if not licenses:
        audit.fail(f"{label}: extracted installer contains no LICENSE resource")
    elif not any(same_text_ignoring_crlf(path, audit.license_path) for path in licenses):
        audit.fail(f"{label}: extracted installer LICENSE does not match repository LICENSE")
    allowed = verify_windows_runtime(audit, root, label)
    if not any(
        (path.parent / "gstreamer-1.0-0.dll").is_file()
        and (path.parent / "lib/gstreamer-1.0").is_dir()
        and (path.parent / "third-party/gstreamer/manifest.json").is_file()
        for path in executables
    ):
        audit.fail(f"{label}: private GStreamer runtime is not installed beside cassette.exe")
    for executable in executables:
        verify_windows_binary_imports(audit, root, executable, allowed, label)
    return allowed


def audit_artifact(audit: Audit, path: Path, temporary_root: Path) -> None:
    if not path.exists():
        audit.fail(f"Required artifact does not exist: {path}")
        return

    kind = detect_kind(path)
    label = path.name
    print(f"Auditing {kind}: {path}")
    if kind in {"appimage", "deb", "rpm", "nsis", "msi"} and audit.expected_version not in path.name:
        audit.fail(f"{label}: filename does not contain exact version {audit.expected_version}")

    destination = temporary_root / f"{len(list(temporary_root.iterdir())):02d}-{kind}"
    destination.mkdir()

    try:
        if kind == "signature":
            validate_signature(path, audit.expected_version)
            audit.scanned_files += 1
        elif kind == "deb":
            root, control = extract_deb(path, destination)
            verify_deb(audit, path, root, control)
            scan_tree(audit, root, label)
        elif kind == "rpm":
            root = extract_rpm(path, destination)
            verify_rpm(audit, path, root)
            scan_tree(audit, root, label)
        elif kind == "appimage":
            if not file_is_x86_64_elf(path):
                audit.fail(f"{label}: AppImage runtime is not x86_64 ELF")
            root = extract_appimage(path, destination)
            verify_appdir(audit, root, label)
            scan_tree(audit, root, label)
        elif kind == "appdir":
            verify_appdir(audit, path, label)
            scan_tree(audit, path, label)
        elif kind == "nsis":
            root = extract_nsis(path, destination)
            allowed = verify_windows_payload(audit, root, label)
            scan_tree(audit, root, label, windows_bundle=True, allowed_windows_runtime=allowed)
        elif kind == "msi":
            root = extract_msi(path, destination)
            if root is None:
                audit.warn(f"{label}: MSI extraction is unsupported on this host; container scan only")
            else:
                allowed = verify_windows_payload(audit, root, label)
                scan_tree(audit, root, label, windows_bundle=True, allowed_windows_runtime=allowed)
        elif kind in {"directory"}:
            scan_tree(audit, path, label)
        else:
            if audit.expected_version.encode() not in path.read_bytes():
                audit.fail(f"{label}: native executable does not contain expected version")
            if path.suffix.lower() == ".exe":
                if not file_is_x86_64_pe(path):
                    audit.fail(f"{label}: native executable is not x86_64 PE")
                elif not {3, 14}.issubset(pe_resource_types(path)):
                    audit.fail(f"{label}: native executable has no complete Windows icon resource")
            elif path.read_bytes()[:4] == b"\x7fELF" and not file_is_x86_64_elf(path):
                audit.fail(f"{label}: native executable is not x86_64 ELF")
            patterns = content_patterns(audit)
            scan_file(
                audit,
                path,
                Path(path.name),
                label,
                frontend=False,
                native=True,
                scan_secrets=True,
                patterns=patterns,
            )
    except (OSError, RuntimeError, tarfile.TarError, ValueError) as error:
        audit.fail(f"{label}: inspection failed: {error}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--workspace", type=Path, default=Path.cwd())
    parser.add_argument("--version", default=EXPECTED_VERSION)
    parser.add_argument("--license", type=Path, default=Path("LICENSE"))
    parser.add_argument("--frontend", action="append", type=Path, default=[])
    parser.add_argument("--artifact", action="append", type=Path, default=[])
    parser.add_argument("--installed-windows-dir", type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    workspace = args.workspace.resolve()
    license_path = args.license if args.license.is_absolute() else workspace / args.license
    if not license_path.is_file():
        print(f"ERROR: license file does not exist: {license_path}", file=sys.stderr)
        return 2

    audit = Audit(workspace, args.version, license_path.resolve())
    if audit.expected_version != EXPECTED_VERSION:
        audit.fail(f"Checkpoint 2A accepts only version {EXPECTED_VERSION}")

    audit_configuration(audit)
    for frontend in args.frontend:
        frontend_path = frontend if frontend.is_absolute() else workspace / frontend
        scan_frontend(audit, frontend_path.resolve())

    if args.installed_windows_dir is not None:
        installed = args.installed_windows_dir.resolve()
        if not installed.is_dir():
            audit.fail(f"Windows installer did not create the expected directory: {installed}")
        else:
            allowed = verify_windows_payload(audit, installed, "installed Windows package")
            scan_tree(
                audit, installed, "installed Windows package",
                windows_bundle=True, allowed_windows_runtime=allowed,
            )

    with tempfile.TemporaryDirectory(prefix="cassette-artifact-audit-") as temporary:
        temporary_root = Path(temporary)
        for artifact in args.artifact:
            artifact_path = artifact if artifact.is_absolute() else workspace / artifact
            # Preserve the signature's original filename and symlink identity.
            audit_path = artifact_path.absolute() if artifact_path.suffix.lower() == ".sig" else artifact_path.resolve()
            audit_artifact(audit, audit_path, temporary_root)

    for warning in sorted(audit.warnings):
        print(f"WARNING: {warning}")
    if audit.native_dev_url_exceptions:
        print(
            "NOTE: allowed "
            f"{audit.native_dev_url_exceptions} exact Tauri localhost:1420 native-string occurrence(s); "
            "built frontend and production CSP remain strict."
        )
    for error in sorted(audit.errors):
        print(f"ERROR: {error}", file=sys.stderr)

    if audit.errors:
        print(
            f"Artifact audit failed: {len(audit.errors)} issue(s) across {audit.scanned_files} files.",
            file=sys.stderr,
        )
        return 1
    print(f"Artifact audit passed: {audit.scanned_files} files checked.")
    print(f"LICENSE SHA-256: {audit.license_hash}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
