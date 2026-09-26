param(
  [string]$GStreamerRoot = $env:GSTREAMER_1_0_ROOT_MSVC_X86_64,
  [string]$Destination = "src-tauri/windows-runtime"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$version = "1.26.11"
$runtimeMsiSha256 = "31cbc21fa0950b5c1e79c80959b2799805cb05a7a35953a13a9f790776137605"
$sourceUrl = "https://gstreamer.freedesktop.org/pkg/windows/$version/msvc/gstreamer-1.0-msvc-x86_64-$version.msi"
if (-not $GStreamerRoot -or -not [IO.Path]::IsPathRooted($GStreamerRoot)) {
  throw "An absolute, verified GStreamer MSVC x86_64 root is required."
}
$root = [IO.Path]::GetFullPath($GStreamerRoot)
$bin = Join-Path $root "bin"
$plugins = Join-Path $root "lib/gstreamer-1.0"
$inspect = Join-Path $bin "gst-inspect-1.0.exe"
foreach ($required in @($bin, $plugins, $inspect)) {
  if (-not (Test-Path -LiteralPath $required)) { throw "Missing GStreamer runtime path: $required" }
}

$output = [IO.Path]::GetFullPath($Destination)
if (Test-Path -LiteralPath $output) {
  throw "Refusing to overwrite an existing runtime staging directory: $output"
}
New-Item -ItemType Directory -Path $output | Out-Null
$staged = [System.Collections.Generic.List[object]]::new()
function Add-StagedFile {
  param([string]$Source, [string]$Subdirectory, [string]$InstalledPath)
  if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) { throw "Missing staged source: $Source" }
  $targetDirectory = Join-Path $output $Subdirectory
  New-Item -ItemType Directory -Force -Path $targetDirectory | Out-Null
  $target = Join-Path $targetDirectory (Split-Path -Leaf $Source)
  Copy-Item -LiteralPath $Source -Destination $target
  $staged.Add([ordered]@{
    path = $InstalledPath.Replace('\', '/')
    sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $target).Hash.ToLowerInvariant()
  })
}

# Resolve the actual element providers rather than guessing plug-in filenames.
# The official MSI can reorganize elements between plug-ins in a patch release.
$elements = @(
  "playbin", "uridecodebin", "decodebin3", "filesrc", "typefind",
  "audioconvert", "audioresample", "autoaudiosink", "id3demux",
  "flacdec", "oggdemux", "vorbisdec", "opusdec", "wavparse",
  "qtdemux", "avdec_mp3", "avdec_aac", "wasapi2sink", "wasapisink",
  "flacparse", "mpegaudioparse", "aacparse", "vorbisparse", "opusparse"
)
$selectedPlugins = [System.Collections.Generic.Dictionary[string,string]]::new([StringComparer]::OrdinalIgnoreCase)
$elementProviders = [ordered]@{}
$env:PATH = "$bin;$env:PATH"
$env:GST_PLUGIN_PATH_1_0 = $plugins
function Select-PluginProvider {
  param([string]$Feature, [switch]$RequireHardwareSink)
  $details = (& $inspect $Feature 2>&1 | Out-String)
  if ($LASTEXITCODE -ne 0) { throw "Required GStreamer feature unavailable: $Feature`n$details" }
  if ($RequireHardwareSink -and $details -notmatch '(?m)^\s*Klass\s+Sink/Audio/Hardware\s*$') {
    throw "$Feature is not a Windows hardware audio sink"
  }
  $filenameMatch = [regex]::Match($details, '(?m)^\s*Filename\s+(.+?)\s*$')
  $licenseMatch = [regex]::Match($details, '(?m)^\s*License\s+(.+?)\s*$')
  if (-not $filenameMatch.Success -or -not $licenseMatch.Success) {
    throw "Could not read provider and license for GStreamer feature $Feature"
  }
  $provider = [IO.Path]::GetFullPath($filenameMatch.Groups[1].Value.Trim())
  if (-not $provider.StartsWith("$plugins\", [StringComparison]::OrdinalIgnoreCase) -or
      -not $provider.EndsWith(".dll", [StringComparison]::OrdinalIgnoreCase)) {
    throw "Feature $Feature is not provided by a DLL in the verified runtime: $provider"
  }
  $license = $licenseMatch.Groups[1].Value.Trim()
  if ($license -notmatch '^(LGPL(?:[- .0-9+]*)?|MIT|BSD(?:[- .0-9+]*)?)$') {
    throw "Refusing a non-allowlisted GStreamer plugin license for $Feature`: $license"
  }
  $selectedPlugins[$provider] = $license
  return (Split-Path -Leaf $provider)
}
foreach ($element in $elements) {
  $elementProviders[$element] = Select-PluginProvider $element -RequireHardwareSink:($element -in @("wasapi2sink", "wasapisink"))
}
# The typefind *element* comes from coreelements; media signatures are supplied
# by a distinct plugin containing GstTypeFindFactory registrations.
$typefindProvider = Select-PluginProvider "typefindfunctions"
if ($typefindProvider -ine "gsttypefindfunctions.dll") {
  throw "Unexpected typefindfunctions plugin provider: $typefindProvider"
}

$scanner = Get-ChildItem -LiteralPath $root -Filter "gst-plugin-scanner.exe" -File -Recurse |
  Select-Object -First 1
if (-not $scanner) { throw "The official runtime has no gst-plugin-scanner.exe" }

# dumpbin is part of the already-installed MSVC build tools. Follow imported
# DLLs transitively, but copy only dependencies supplied by the verified MSI.
$vswhere = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio/Installer/vswhere.exe"
if (-not (Test-Path -LiteralPath $vswhere)) { throw "vswhere.exe is unavailable" }
$visualStudio = (& $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath | Select-Object -First 1)
if (-not $visualStudio) { throw "MSVC build tools are unavailable" }
$dumpbin = Get-ChildItem -LiteralPath (Join-Path $visualStudio "VC/Tools/MSVC") -Filter dumpbin.exe -File -Recurse |
  Where-Object { $_.FullName -match 'Hostx64[\\/]x64[\\/]dumpbin\.exe$' } |
  Sort-Object FullName -Descending | Select-Object -First 1
if (-not $dumpbin) { throw "MSVC dumpbin.exe is unavailable" }

# The pinned Tauri version statically links its own VC runtime, but the
# official GStreamer MSVC binaries import the dynamic VC runtime. Stage only
# the DLLs actually imported, from Visual Studio's licensed x64 redist set.
$redistRoot = Join-Path $visualStudio "VC/Redist/MSVC"
$vcRedist = Get-ChildItem -LiteralPath $redistRoot -Directory -ErrorAction Stop |
  Sort-Object Name -Descending |
  ForEach-Object { Join-Path $_.FullName "x64/Microsoft.VC143.CRT" } |
  Where-Object { Test-Path -LiteralPath (Join-Path $_ "vcruntime140.dll") -PathType Leaf } |
  Select-Object -First 1
if (-not $vcRedist) { throw "The Microsoft Visual C++ x64 redistributable DLLs are unavailable" }
$availableVcDlls = [System.Collections.Generic.Dictionary[string,string]]::new([StringComparer]::OrdinalIgnoreCase)
Get-ChildItem -LiteralPath $vcRedist -Filter "*.dll" -File | ForEach-Object {
  if ($_.Name -match '^(VCRUNTIME140(_1)?|MSVCP140(_1|_2|_atomic_wait|_codecvt_ids)?|CONCRT140)\.dll$') {
    $availableVcDlls[$_.Name] = $_.FullName
  }
}
$selectedVcDlls = [System.Collections.Generic.Dictionary[string,string]]::new([StringComparer]::OrdinalIgnoreCase)

$availableDlls = [System.Collections.Generic.Dictionary[string,string]]::new([StringComparer]::OrdinalIgnoreCase)
Get-ChildItem -LiteralPath $bin -Filter "*.dll" -File | ForEach-Object {
  $availableDlls[$_.Name] = $_.FullName
}
$pending = [System.Collections.Generic.Queue[string]]::new()
$pending.Enqueue((Join-Path $bin "gstreamer-1.0-0.dll"))
$pending.Enqueue($scanner.FullName)
$pending.Enqueue($inspect)
foreach ($plugin in $selectedPlugins.Keys) { $pending.Enqueue($plugin) }
$selectedBinDlls = [System.Collections.Generic.Dictionary[string,string]]::new([StringComparer]::OrdinalIgnoreCase)
# cassette.exe links GIO directly. Its imports are resolved by the Windows
# loader before main(), so process-local PATH setup cannot rescue a missing
# DLL. The packaged executable's complete import closure is checked again
# after NSIS installation; this seed makes the current native link explicit.
if (-not $availableDlls.ContainsKey("gio-2.0-0.dll")) {
  throw "The verified GStreamer runtime is missing Cassette's direct GIO dependency"
}
$selectedBinDlls["gio-2.0-0.dll"] = $availableDlls["gio-2.0-0.dll"]
$pending.Enqueue($availableDlls["gio-2.0-0.dll"])
$visited = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
while ($pending.Count -gt 0) {
  $current = $pending.Dequeue()
  if (-not (Test-Path -LiteralPath $current -PathType Leaf)) { throw "Missing dependency: $current" }
  if (-not $visited.Add($current)) { continue }
  $dependencies = (& $dumpbin.FullName /DEPENDENTS $current 2>&1 | Out-String)
  if ($LASTEXITCODE -ne 0) { throw "Could not inspect PE dependencies of $current`: $dependencies" }
  foreach ($match in [regex]::Matches($dependencies, '(?m)^\s*([A-Za-z0-9_.+-]+\.dll)\s*$')) {
    $name = $match.Groups[1].Value
    if ($availableDlls.ContainsKey($name)) {
      $dependency = $availableDlls[$name]
      $selectedBinDlls[$name] = $dependency
      $pending.Enqueue($dependency)
    } elseif ($name -match '^(api-ms-win-|ext-ms-win-)') {
      continue
    } elseif ($name -match '^(VCRUNTIME140(_1)?|MSVCP140(_1|_2|_atomic_wait|_codecvt_ids)?|CONCRT140)\.dll$') {
      if (-not $availableVcDlls.ContainsKey($name)) {
        throw "VC runtime dependency $name for $current is absent from the Visual Studio x64 redist set"
      }
      $dependency = $availableVcDlls[$name]
      $signature = Get-AuthenticodeSignature -LiteralPath $dependency
      if ($signature.Status -ne 'Valid' -or
          -not $signature.SignerCertificate -or
          $signature.SignerCertificate.Subject -notmatch 'Microsoft Corporation') {
        throw "VC runtime dependency $name does not have a valid Microsoft signature"
      }
      $selectedVcDlls[$name] = $dependency
      $selectedBinDlls[$name] = $dependency
      $pending.Enqueue($dependency)
    } elseif (-not (Test-Path -LiteralPath (Join-Path ([Environment]::SystemDirectory) $name))) {
      throw "Non-system PE dependency $name for $current is absent from the official runtime"
    }
  }
}
if (-not $selectedBinDlls.ContainsKey("gstreamer-1.0-0.dll")) {
  $selectedBinDlls["gstreamer-1.0-0.dll"] = Join-Path $bin "gstreamer-1.0-0.dll"
}
if (-not $selectedVcDlls.ContainsKey("vcruntime140.dll")) {
  throw "GStreamer unexpectedly does not import vcruntime140.dll; recheck the private runtime staging policy"
}

foreach ($name in ($selectedBinDlls.Keys | Sort-Object)) {
  # dumpbin preserves an import's spelling (often uppercase), while the
  # redistributable's actual filename may be lowercase. Record the installed
  # filename so the manifest also audits on case-sensitive hosts.
  Add-StagedFile $selectedBinDlls[$name] "bin" (Split-Path -Leaf $selectedBinDlls[$name])
}
foreach ($source in ($selectedPlugins.Keys | Sort-Object)) {
  $name = Split-Path -Leaf $source
  Add-StagedFile $source "plugins" "lib/gstreamer-1.0/$name"
}
Add-StagedFile $scanner.FullName "scanner" "libexec/gstreamer-1.0/gst-plugin-scanner.exe"

# Preserve the license texts shipped by the official runtime. Flatten names
# deterministically because Tauri's resource glob flattens the destination.
$licenseSources = @(Get-ChildItem -LiteralPath $root -Recurse -File | Where-Object {
  $_.Name -match '^(COPYING|LICENSE|NOTICE)([._-].*)?$' -and
  $_.FullName -notmatch '[\\/](include|pkgconfig)[\\/]'
})
if (-not $licenseSources) { throw "The official runtime has no redistributable license texts" }
$hasLgpl = $false
foreach ($source in $licenseSources) {
  $body = [IO.File]::ReadAllText($source.FullName)
  if ($body -match 'GNU LESSER GENERAL PUBLIC LICENSE') { $hasLgpl = $true }
  $relative = [IO.Path]::GetRelativePath($root, $source.FullName)
  $flatName = (($relative -replace '[\\/]', '-') -replace '[^A-Za-z0-9._-]', '_') + ".txt"
  $noticeDirectory = Join-Path $output "notices"
  New-Item -ItemType Directory -Force -Path $noticeDirectory | Out-Null
  $target = Join-Path $noticeDirectory $flatName
  if (Test-Path -LiteralPath $target) { throw "License notice filename collision: $relative" }
  Copy-Item -LiteralPath $source.FullName -Destination $target
  $staged.Add([ordered]@{
    path = "third-party/gstreamer/$flatName"
    sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $target).Hash.ToLowerInvariant()
  })
}
if (-not $hasLgpl) { throw "The official runtime did not provide the LGPL text" }

$notice = @(
  "Cassette diagnostic Windows installer: private GStreamer $version runtime",
  "Official runtime MSI: $sourceUrl",
  "Official runtime MSI SHA-256: $runtimeMsiSha256",
  "GStreamer source: https://gitlab.freedesktop.org/gstreamer/gstreamer/-/tree/$version",
  "Only the DLLs and plugins named in manifest.json are bundled. No development files or shared GStreamer installation are included.",
  "GStreamer and its dependencies retain their upstream licenses. Accompanying upstream license texts are in this directory.",
  "Microsoft VC runtime DLLs are copied only from Visual Studio's Microsoft.VC143.CRT x64 redistributable directory after Authenticode verification.",
  "Microsoft redistribution terms and DLL list: https://learn.microsoft.com/en-us/visualstudio/releases/2022/redistribution and https://learn.microsoft.com/en-us/cpp/windows/determining-which-dlls-to-redistribute",
  "Microsoft VC runtime files in this installer: $((($selectedVcDlls.Keys | Sort-Object) -join ', '))",
  "Codec patent and redistribution review, plus clean Windows 10/11 desktop qualification, remain required before a Windows release.",
  "Selected plugins and reported licenses:"
)
foreach ($source in ($selectedPlugins.Keys | Sort-Object)) {
  $notice += "$(Split-Path -Leaf $source): $($selectedPlugins[$source])"
}
$noticePath = Join-Path $output "notices/NOTICE.txt"
[IO.File]::WriteAllLines($noticePath, $notice, [Text.UTF8Encoding]::new($false))
$staged.Add([ordered]@{
  path = "third-party/gstreamer/NOTICE.txt"
  sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $noticePath).Hash.ToLowerInvariant()
})

$manifest = [ordered]@{
  schemaVersion = 1
  gstreamerVersion = $version
  runtimeMsiUrl = $sourceUrl
  runtimeMsiSha256 = $runtimeMsiSha256
  vcRuntime = [ordered]@{
    source = "Microsoft.VC143.CRT x64"
    dlls = @($selectedVcDlls.Values | ForEach-Object { Split-Path -Leaf $_ } | Sort-Object)
  }
  elementProviders = $elementProviders
  typefindProvider = $typefindProvider
  files = @($staged | Sort-Object { $_.path })
}
$manifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $output "manifest.json") -Encoding utf8
Write-Host "Staged $($selectedBinDlls.Count) runtime DLLs (including $($selectedVcDlls.Count) Microsoft VC redist DLLs), $($selectedPlugins.Count) plugins, scanner, and notices at $output"
