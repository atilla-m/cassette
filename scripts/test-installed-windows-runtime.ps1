param(
  [string]$Installer = "src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/Cassette_0.1.0-beta.3_x64-setup.exe"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$installerPath = [IO.Path]::GetFullPath($Installer)
if (-not (Test-Path -LiteralPath $installerPath -PathType Leaf)) {
  throw "Missing diagnostic NSIS installer: $installerPath"
}
$python = (Get-Command python.exe -ErrorAction Stop).Source
$sourceGstBin = Join-Path $env:GSTREAMER_1_0_ROOT_MSVC_X86_64 "bin"
$sourceInspect = Join-Path $sourceGstBin "gst-inspect-1.0.exe"
$sourceLaunch = Join-Path $sourceGstBin "gst-launch-1.0.exe"
foreach ($tool in @($sourceInspect, $sourceLaunch)) {
  if (-not (Test-Path -LiteralPath $tool -PathType Leaf)) { throw "Missing source test tool: $tool" }
}
$installRoot = Join-Path $env:RUNNER_TEMP "cassette-installed-runtime-smoke"
if (Test-Path -LiteralPath $installRoot) {
  throw "Refusing to reuse the installed-package test directory: $installRoot"
}

# Exercise the actual NSIS installation layout, not the staging directory or
# 7-Zip's approximation of it. /D is NSIS's last argument and uses a runner-
# local path with no spaces. This CI runner is disposable.
$installation = Start-Process -FilePath $installerPath -ArgumentList @("/S", "/D=$installRoot") -Wait -PassThru
if ($installation.ExitCode -ne 0) {
  throw "Diagnostic NSIS installation failed with exit code $($installation.ExitCode)"
}
$application = Join-Path $installRoot "cassette.exe"
if (-not (Test-Path -LiteralPath $application -PathType Leaf)) {
  throw "NSIS did not install Cassette at the isolated destination: $application"
}

# The builder has a globally installed GStreamer MSI. Exclude it, its plugin
# variables, and developer tool directories before auditing or launching the
# installed application. Windows still searches the EXE's own directory.
$env:PATH = "$env:SystemRoot\System32;$env:SystemRoot"
foreach ($name in @(
  "GSTREAMER_1_0_ROOT_MSVC_X86_64", "GSTREAMER_ROOT_X86_64",
  "GST_PLUGIN_PATH_1_0", "GST_PLUGIN_SYSTEM_PATH_1_0",
  "GST_PLUGIN_SCANNER_1_0", "GST_REGISTRY_1_0", "PKG_CONFIG_PATH"
)) {
  Remove-Item -Path "Env:$name" -ErrorAction SilentlyContinue
}

& $python scripts/audit-release-artifacts.py `
  --workspace . --version 0.1.0-beta.3 --license LICENSE `
  --installed-windows-dir $installRoot
if ($LASTEXITCODE -ne 0) {
  throw "The installed package has an unresolved or unlisted DLL dependency"
}

# Copy only the official MSI's test executables, never its libraries or plugins.
# With the copied tools outside the MSI bin directory and PATH restricted to the
# installed app, Windows can resolve GStreamer DLLs only from the NSIS payload.
$testRoot = Join-Path $env:RUNNER_TEMP "cassette-installed-playback-smoke"
if (Test-Path -LiteralPath $testRoot) { throw "Refusing to reuse playback test directory: $testRoot" }
New-Item -ItemType Directory -Path $testRoot | Out-Null
Copy-Item -LiteralPath $sourceInspect, $sourceLaunch -Destination $testRoot
$inspect = Join-Path $testRoot "gst-inspect-1.0.exe"
$launch = Join-Path $testRoot "gst-launch-1.0.exe"
$fixtureRoot = Join-Path $testRoot "media"
New-Item -ItemType Directory -Path $fixtureRoot | Out-Null
foreach ($extension in @("flac", "mp3", "ogg", "opus", "m4a")) {
  $encoded = (Get-Content -LiteralPath "src-tauri/test-fixtures/tag-edit-tone.$extension.base64" -Raw).Trim()
  [IO.File]::WriteAllBytes((Join-Path $fixtureRoot "tone.$extension"), [Convert]::FromBase64String($encoded))
}
$wav = [IO.MemoryStream]::new()
$writer = [IO.BinaryWriter]::new($wav)
$writer.Write([Text.Encoding]::ASCII.GetBytes("RIFF"))
$writer.Write([uint32]1636)
$writer.Write([Text.Encoding]::ASCII.GetBytes("WAVEfmt "))
$writer.Write([uint32]16)
$writer.Write([uint16]1)
$writer.Write([uint16]1)
$writer.Write([uint32]8000)
$writer.Write([uint32]16000)
$writer.Write([uint16]2)
$writer.Write([uint16]16)
$writer.Write([Text.Encoding]::ASCII.GetBytes("data"))
$writer.Write([uint32]1600)
$writer.Write([byte[]]::new(1600))
[IO.File]::WriteAllBytes((Join-Path $fixtureRoot "tone.wav"), $wav.ToArray())
$writer.Dispose()
$wav.Dispose()

$env:PATH = "$installRoot;$env:SystemRoot\System32;$env:SystemRoot"
$env:GST_PLUGIN_PATH_1_0 = Join-Path $installRoot "lib/gstreamer-1.0"
$env:GST_PLUGIN_SYSTEM_PATH_1_0 = ""
$env:GST_PLUGIN_SCANNER_1_0 = Join-Path $installRoot "libexec/gstreamer-1.0/gst-plugin-scanner.exe"
$env:GST_REGISTRY_1_0 = Join-Path $testRoot "fresh-registry.bin"
if (Test-Path -LiteralPath $env:GST_REGISTRY_1_0) { throw "Playback registry is not fresh" }
$typefind = & $inspect typefindfunctions 2>&1 | Out-String
if ($LASTEXITCODE -ne 0 -or $typefind -notmatch 'typefindfunctions') {
  throw "Installed package has no usable typefindfunctions plugin: $typefind"
}
foreach ($sink in @("wasapi2sink", "wasapisink")) {
  $details = & $inspect $sink 2>&1 | Out-String
  if ($LASTEXITCODE -ne 0 -or $details -notmatch '(?m)^\s*Klass\s+Sink/Audio/Hardware\s*$') {
    throw "Installed package has no loadable Windows hardware sink factory $sink`: $details"
  }
}
foreach ($extension in @("flac", "mp3", "ogg", "opus", "wav", "m4a")) {
  $file = Join-Path $fixtureRoot "tone.$extension"
  $decode = & $launch -m -v filesrc "location=$file" ! decodebin ! audioconvert ! audioresample ! fakesink sync=false 2>&1 | Out-String
  if ($LASTEXITCODE -ne 0 -or $decode -notmatch 'audio/x-raw' -or $decode -notmatch 'Got EOS from element') {
    throw "Installed package could not typefind and decode $extension to raw audio/EOS: $decode"
  }
  Write-Host "Installed package detected and decoded synthetic $extension to raw audio/EOS."
}
Write-Host "Installed-package six-format decoding and Windows audio-output factory checks passed using only bundled DLLs/plugins and a fresh registry; audible output was not tested."

# A missing loader-time DLL exits before Cassette's main(). A short process
# survival check catches that failure without claiming desktop/UI qualification.
$running = Start-Process -FilePath $application -WorkingDirectory $installRoot -PassThru
try {
  if ($running.WaitForExit(8000)) {
    throw "Installed Cassette exited during startup (exit code $($running.ExitCode))"
  }
  Write-Host "Installed-package DLL closure and loader startup passed without build-machine GStreamer on PATH."
} finally {
  if (-not $running.HasExited) {
    Stop-Process -Id $running.Id -Force
    $running.WaitForExit()
  }
}
