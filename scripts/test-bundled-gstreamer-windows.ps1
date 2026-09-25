param([string]$StagedRuntime = "src-tauri/windows-runtime")

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$stage = [IO.Path]::GetFullPath($StagedRuntime)
$sourceInspect = Join-Path $env:GSTREAMER_1_0_ROOT_MSVC_X86_64 "bin/gst-inspect-1.0.exe"
if (-not (Test-Path -LiteralPath $sourceInspect)) { throw "Missing source gst-inspect-1.0.exe" }
$testRoot = Join-Path $env:RUNNER_TEMP "cassette-isolated-gstreamer-smoke"
if (Test-Path -LiteralPath $testRoot) { throw "Refusing to reuse isolated runtime test directory: $testRoot" }
New-Item -ItemType Directory -Path $testRoot | Out-Null
Copy-Item -Path (Join-Path $stage "bin/*.dll") -Destination $testRoot
Copy-Item -LiteralPath $sourceInspect -Destination $testRoot
$pluginDirectory = Join-Path $testRoot "lib/gstreamer-1.0"
$scannerDirectory = Join-Path $testRoot "libexec/gstreamer-1.0"
New-Item -ItemType Directory -Path $pluginDirectory, $scannerDirectory -Force | Out-Null
Copy-Item -Path (Join-Path $stage "plugins/*.dll") -Destination $pluginDirectory
Copy-Item -LiteralPath (Join-Path $stage "scanner/gst-plugin-scanner.exe") -Destination $scannerDirectory

# Deliberately exclude the globally installed GStreamer from this process.
# The Windows system and VC runtimes are normal OS/installer prerequisites.
$env:PATH = "$testRoot;$env:SystemRoot\System32;$env:SystemRoot"
$env:GST_PLUGIN_PATH_1_0 = $pluginDirectory
$env:GST_PLUGIN_SYSTEM_PATH_1_0 = ""
$env:GST_PLUGIN_SCANNER_1_0 = Join-Path $scannerDirectory "gst-plugin-scanner.exe"
$env:GST_REGISTRY_1_0 = Join-Path $testRoot "registry.bin"
$manifest = Get-Content -LiteralPath (Join-Path $stage "manifest.json") -Raw | ConvertFrom-Json
foreach ($element in $manifest.elementProviders.PSObject.Properties.Name) {
  $result = & (Join-Path $testRoot "gst-inspect-1.0.exe") $element 2>&1 | Out-String
  if ($LASTEXITCODE -ne 0) { throw "Isolated GStreamer cannot load $element`: $result" }
}
Write-Host "Isolated runtime loaded all $($manifest.elementProviders.PSObject.Properties.Count) required elements without system GStreamer on PATH."
