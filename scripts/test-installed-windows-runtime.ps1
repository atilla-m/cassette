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
