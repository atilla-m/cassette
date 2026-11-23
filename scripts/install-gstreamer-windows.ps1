param(
  [string]$InstallRoot = (Join-Path $env:SystemDrive "cassette-build-tools\gstreamer")
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$release = Get-Content -LiteralPath (Join-Path $PSScriptRoot "windows-gstreamer.json") -Raw | ConvertFrom-Json
$version = $release.version
$source = [Uri]$release.installerUrl
$root = [IO.Path]::GetFullPath($InstallRoot)
$temporaryRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [IO.Path]::GetTempPath() }
$downloadDirectory = Join-Path $temporaryRoot "cassette-gstreamer-$version"
New-Item -ItemType Directory -Force -Path $downloadDirectory | Out-Null
$installer = Join-Path $downloadDirectory ([IO.Path]::GetFileName($source.AbsolutePath))
$installLog = Join-Path $downloadDirectory "install.log"
Invoke-WebRequest -Uri $source -OutFile $installer

$actualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $installer).Hash
if ($actualHash -ne $release.installerSha256) {
  throw "GStreamer $version installer checksum mismatch: $actualHash"
}

$process = Start-Process -FilePath $installer -Wait -PassThru -WindowStyle Hidden -ArgumentList @(
  "/CURRENTUSER",
  "/portable=1",
  "/TYPE=devel",
  "/VERYSILENT",
  "/SUPPRESSMSGBOXES",
  "/NORESTART",
  ('/DIR="{0}"' -f $root),
  ('/LOG="{0}"' -f $installLog)
)
if ($process.ExitCode -ne 0) {
  throw "GStreamer $version installation exited with code $($process.ExitCode). Log: $installLog"
}

$binDirectory = Join-Path $root "bin"
$pkgConfigDirectory = Join-Path $root "lib\pkgconfig"
$pluginDirectory = Join-Path $root "lib\gstreamer-1.0"
$gstInspect = Join-Path $binDirectory "gst-inspect-1.0.exe"
$pkgConfigExecutable = @(
  (Join-Path $binDirectory "pkg-config.exe"),
  (Join-Path $binDirectory "pkgconf.exe")
) | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } | Select-Object -First 1

if (-not $pkgConfigExecutable -or
    -not (Test-Path -LiteralPath $gstInspect -PathType Leaf) -or
    -not (Test-Path -LiteralPath (Join-Path $pkgConfigDirectory "gstreamer-1.0.pc") -PathType Leaf) -or
    -not (Test-Path -LiteralPath $pluginDirectory -PathType Container)) {
  throw "GStreamer runtime or development components are missing at $root. Log: $installLog"
}

$env:PATH = "$binDirectory;$env:PATH"
$inspectOutput = (& $gstInspect --version 2>&1 | Out-String)
if ($LASTEXITCODE -ne 0 -or $inspectOutput -notmatch "(?m)^GStreamer $([regex]::Escape($version))\s*$") {
  throw "Expected GStreamer $version at $gstInspect. Output: $inspectOutput"
}
Write-Host $inspectOutput.Trim()

$environmentValues = [ordered]@{
  "GSTREAMER_ROOT_X86_64" = $root
  "GSTREAMER_1_0_ROOT_MSVC_X86_64" = $root
  "PKG_CONFIG_PATH" = $pkgConfigDirectory
  "PKG_CONFIG" = $pkgConfigExecutable
  "GST_PLUGIN_PATH_1_0" = $pluginDirectory
}
if ($env:GITHUB_ENV) {
  foreach ($entry in $environmentValues.GetEnumerator()) {
    "$($entry.Key)=$($entry.Value)" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
  }
}
if ($env:GITHUB_PATH) {
  $binDirectory | Out-File -FilePath $env:GITHUB_PATH -Encoding utf8 -Append
}

Write-Host "Installed and verified official GStreamer MSVC x86_64 $version at $root"
