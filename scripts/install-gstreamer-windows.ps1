param(
  [string]$InstallRoot = "C:\gstreamer"
)

Set-StrictMode -Version Latest
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

$temporaryRoot = if ($env:RUNNER_TEMP) {
  $env:RUNNER_TEMP
} else {
  [System.IO.Path]::GetTempPath()
}
$downloadDirectory = Join-Path $temporaryRoot "cassette-gstreamer-$version"
New-Item -ItemType Directory -Force -Path $downloadDirectory | Out-Null

foreach ($package in $packages) {
  $installer = Join-Path $downloadDirectory $package.Name
  $installLog = Join-Path $downloadDirectory "$($package.Name).log"
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
    "/l*v",
    "`"$installLog`"",
    "INSTALLDIR=`"$InstallRoot`"",
    "ADDLOCAL=ALL"
  )
  if ($process.ExitCode -notin @(0, 3010)) {
    throw "GStreamer installer $($package.Name) exited with code $($process.ExitCode). MSI log: $installLog"
  }
  Write-Host "Verified $($package.Name); msiexec exit code $($process.ExitCode)."
}

$candidateRoots = [System.Collections.Generic.List[string]]::new()
function Add-CandidateRoot {
  param([string]$Path)

  if ([string]::IsNullOrWhiteSpace($Path)) {
    return
  }
  $expanded = [Environment]::ExpandEnvironmentVariables($Path.Trim('"'))
  if (-not $candidateRoots.Contains($expanded)) {
    $candidateRoots.Add($expanded) | Out-Null
  }
}

# First check requested and documented locations, then installer registry data.
Add-CandidateRoot $InstallRoot
Add-CandidateRoot (Join-Path $InstallRoot "1.0\msvc_x86_64")
Add-CandidateRoot $env:GSTREAMER_1_0_ROOT_MSVC_X86_64
Add-CandidateRoot $env:GSTREAMER_ROOT_X86_64
Add-CandidateRoot "C:\gstreamer\1.0\msvc_x86_64"
Add-CandidateRoot (Join-Path $env:ProgramFiles "gstreamer\1.0\msvc_x86_64")
if ($env:LOCALAPPDATA) {
  Add-CandidateRoot (Join-Path $env:LOCALAPPDATA "Programs\gstreamer\1.0\msvc_x86_64")
}
if (${env:ProgramFiles(x86)}) {
  Add-CandidateRoot (Join-Path ${env:ProgramFiles(x86)} "gstreamer\1.0\msvc_x86_64")
}

$uninstallRegistryRoots = @(
  "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*",
  "HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*",
  "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*"
)
foreach ($registryRoot in $uninstallRegistryRoots) {
  $registryItems = Get-ItemProperty -Path $registryRoot -ErrorAction SilentlyContinue
  foreach ($registryItem in $registryItems) {
    $displayNameProperty = $registryItem.PSObject.Properties["DisplayName"]
    if (-not $displayNameProperty) {
      continue
    }

    $displayName = [string]$displayNameProperty.Value
    if ($displayName -like "GStreamer*MSVC*x86_64*" -or $displayName -like "GStreamer*1.26.11*") {
      $installLocationProperty = $registryItem.PSObject.Properties["InstallLocation"]
      if ($installLocationProperty) {
        Add-CandidateRoot ([string]$installLocationProperty.Value)
      }
    }
  }
}

$gstInspectCandidates = [System.Collections.Generic.List[string]]::new()
foreach ($candidateRoot in $candidateRoots) {
  $candidate = Join-Path $candidateRoot "bin\gst-inspect-1.0.exe"
  if ((Test-Path -LiteralPath $candidate) -and -not $gstInspectCandidates.Contains($candidate)) {
    $gstInspectCandidates.Add($candidate) | Out-Null
  }
}

# Some MSI versions ignore or reinterpret INSTALLDIR. Search only documented
# GStreamer roots, never an entire drive or developer profile.
$searchRoots = [System.Collections.Generic.List[string]]::new()
foreach ($searchRoot in @(
  "C:\gstreamer",
  (Join-Path $env:ProgramFiles "gstreamer"),
  $(if ($env:LOCALAPPDATA) { Join-Path $env:LOCALAPPDATA "Programs\gstreamer" }),
  $(if (${env:ProgramFiles(x86)}) { Join-Path ${env:ProgramFiles(x86)} "gstreamer" })
)) {
  if ($searchRoot -and (Test-Path -LiteralPath $searchRoot) -and -not $searchRoots.Contains($searchRoot)) {
    $searchRoots.Add($searchRoot) | Out-Null
  }
}
foreach ($searchRoot in $searchRoots) {
  Get-ChildItem -LiteralPath $searchRoot -Filter "gst-inspect-1.0.exe" -File -Recurse -ErrorAction SilentlyContinue |
    Sort-Object FullName |
    ForEach-Object {
      if (-not $gstInspectCandidates.Contains($_.FullName)) {
        $gstInspectCandidates.Add($_.FullName) | Out-Null
      }
    }
}

$resolvedRoot = $null
$gstInspect = $null
foreach ($candidate in $gstInspectCandidates) {
  $candidateBin = Split-Path -Parent $candidate
  $candidateRoot = Split-Path -Parent $candidateBin
  $candidatePkgConfig = Join-Path $candidateRoot "lib\pkgconfig\gstreamer-1.0.pc"
  if (Test-Path -LiteralPath $candidatePkgConfig) {
    $resolvedRoot = $candidateRoot
    $gstInspect = $candidate
    break
  }
}

if (-not $resolvedRoot -or -not $gstInspect) {
  $checkedRoots = if ($candidateRoots.Count) { $candidateRoots -join "; " } else { "(none)" }
  $foundExecutables = if ($gstInspectCandidates.Count) { $gstInspectCandidates -join "; " } else { "(none)" }
  throw "GStreamer installation is incomplete. Checked roots: $checkedRoots. gst-inspect candidates: $foundExecutables. MSI logs: $downloadDirectory"
}

$binDirectory = Join-Path $resolvedRoot "bin"
$pkgConfigDirectory = Join-Path $resolvedRoot "lib\pkgconfig"
$pluginDirectory = Join-Path $resolvedRoot "lib\gstreamer-1.0"
$pkgConfigExecutable = @(
  (Join-Path $binDirectory "pkg-config.exe"),
  (Join-Path $binDirectory "pkgconf.exe")
) | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1

if (-not $pkgConfigExecutable) {
  throw "GStreamer development files were found at $resolvedRoot, but pkg-config.exe/pkgconf.exe is missing from $binDirectory"
}
if (-not (Test-Path -LiteralPath $pluginDirectory)) {
  throw "GStreamer plugin directory is missing: $pluginDirectory"
}

$env:PATH = "$binDirectory;$env:PATH"
$inspectOutput = & $gstInspect --version 2>&1
if ($LASTEXITCODE -ne 0) {
  throw "gst-inspect-1.0.exe failed with exit code $LASTEXITCODE at $gstInspect. Output: $inspectOutput"
}
Write-Host ($inspectOutput | Out-String).Trim()

$environmentValues = [ordered]@{
  "GSTREAMER_ROOT_X86_64" = $resolvedRoot
  "GSTREAMER_1_0_ROOT_MSVC_X86_64" = $resolvedRoot
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

Write-Host "Installed and verified official GStreamer MSVC x86_64 $version at $resolvedRoot"
Write-Host "Subsequent Actions steps receive PATH, GStreamer roots, pkg-config, and plugin-path settings."
