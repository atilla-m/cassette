param(
  [string]$InstallRoot = "C:\gstreamer\1.0\msvc_x86_64"
)

$ErrorActionPreference = "Stop"

$version = "1.26.11"
$baseUrl = "https://gstreamer.freedesktop.org/pkg/windows/$version/msvc"
$packages = @(
  @{
    Name = "gstreamer-1.0-msvc-x86_64-$version.msi"
    Sha256 = "31cbc21fa0950b5c1e79c80959b2799805cb05a7a35953a13a9f790776137605"
  },
  @{
    Name = "gstreamer-1.0-devel-msvc-x86_64-$version.msi"
    Sha256 = "af0a0720692052aaf5fb0cf2847ef34a87016ef0f914328d516f049e6c86f587"
  }
)

$downloadDirectory = Join-Path $env:RUNNER_TEMP "cassette-gstreamer-$version"
New-Item -ItemType Directory -Force -Path $downloadDirectory | Out-Null

foreach ($package in $packages) {
  $installer = Join-Path $downloadDirectory $package.Name
  Invoke-WebRequest -Uri "$baseUrl/$($package.Name)" -OutFile $installer

  $actualHash = (Get-FileHash -Algorithm SHA256 -Path $installer).Hash.ToLowerInvariant()
  if ($actualHash -ne $package.Sha256) {
    throw "SHA-256 mismatch for $($package.Name): expected $($package.Sha256), got $actualHash"
  }

  $process = Start-Process -FilePath "msiexec.exe" -Wait -PassThru -ArgumentList @(
    "/i",
    "`"$installer`"",
    "/qn",
    "/norestart",
    "INSTALLDIR=`"$InstallRoot`""
  )
  if ($process.ExitCode -notin @(0, 3010)) {
    throw "GStreamer installer $($package.Name) exited with code $($process.ExitCode)"
  }
}

$binDirectory = Join-Path $InstallRoot "bin"
$pkgConfigDirectory = Join-Path $InstallRoot "lib\pkgconfig"
$gstInspect = Join-Path $binDirectory "gst-inspect-1.0.exe"

if (-not (Test-Path $gstInspect)) {
  throw "GStreamer runtime was not found at $gstInspect"
}
if (-not (Test-Path (Join-Path $pkgConfigDirectory "gstreamer-1.0.pc"))) {
  throw "GStreamer development metadata was not found in $pkgConfigDirectory"
}

if ($env:GITHUB_ENV) {
  "GSTREAMER_ROOT_X86_64=$InstallRoot" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
  "GSTREAMER_1_0_ROOT_MSVC_X86_64=$InstallRoot" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
  "PKG_CONFIG_PATH=$pkgConfigDirectory" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
  "PKG_CONFIG=$(Join-Path $binDirectory 'pkg-config.exe')" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
}
if ($env:GITHUB_PATH) {
  $binDirectory | Out-File -FilePath $env:GITHUB_PATH -Encoding utf8 -Append
}

Write-Host "Installed official GStreamer MSVC x86_64 runtime and development files at $InstallRoot"
