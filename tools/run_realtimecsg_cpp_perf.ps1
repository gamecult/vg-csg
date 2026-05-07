param(
    [string]$OutputPath = ".\experiments\generated\realtimecsg-cpp-perf-latest.jsonl",
    [string]$PluginDll = ".\experiments\source-repos\realtime-CSG-for-unity\Plugins\Editor\External\x64\RealtimeCSG[1_559].dll"
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$project = Join-Path $root "tools\realtimecsg_native_bridge"
$resolvedOutput = Join-Path $root $OutputPath
$resolvedDll = Join-Path $root $PluginDll
$releaseDir = Join-Path $project "bin\Release\net10.0"

if (!(Test-Path -LiteralPath $resolvedDll)) {
    throw "RealtimeCSG native DLL not found: $resolvedDll"
}

New-Item -ItemType Directory -Force -Path (Split-Path -Parent $resolvedOutput) | Out-Null
dotnet build $project -c Release | Out-Host
if ($LASTEXITCODE -ne 0) {
    throw "RealtimeCSG native bridge build failed with exit code $LASTEXITCODE"
}
Copy-Item -LiteralPath $resolvedDll -Destination (Join-Path $releaseDir "RealtimeCSG[1_559].dll") -Force
& (Join-Path $releaseDir "RealtimeCsgNativeBridge.exe") $resolvedOutput
if ($LASTEXITCODE -ne 0) {
    Remove-Item -LiteralPath $resolvedOutput -ErrorAction SilentlyContinue
    throw "RealtimeCSG native bridge failed with exit code $LASTEXITCODE"
}
Get-Content -Path $resolvedOutput
