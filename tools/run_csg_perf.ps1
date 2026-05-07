param(
    [string]$OutputPath = ".\experiments\generated\csg-perf-latest.jsonl",
    [string]$ReferenceCommand = $env:VIBEGEOMETRY_REFERENCE_CSG_PERF
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$resolvedOutput = Join-Path $root $OutputPath
$outputDir = Split-Path -Parent $resolvedOutput
New-Item -ItemType Directory -Force -Path $outputDir | Out-Null

Push-Location $root
try {
    cargo build --release -p vg_csg --example csg_perf_fixture | Out-Host

    $exe = Join-Path $root "target\release\examples\csg_perf_fixture.exe"
    if (!(Test-Path $exe)) {
        throw "Missing vg_csg perf fixture executable: $exe"
    }

    & $exe | Set-Content -Path $resolvedOutput -Encoding utf8

    if ($ReferenceCommand) {
        $referenceOutput = & cmd /c $ReferenceCommand
        foreach ($line in $referenceOutput) {
            Add-Content -Path $resolvedOutput -Value $line -Encoding utf8
        }
    } else {
        $missing = @{
            kernel = "reference"
            status = "missing"
            reason = "Set VIBEGEOMETRY_REFERENCE_CSG_PERF or pass -ReferenceCommand with an executable that emits the same JSONL scenario records."
        } | ConvertTo-Json -Compress
        Add-Content -Path $resolvedOutput -Value $missing -Encoding utf8
    }

    Get-Content -Path $resolvedOutput
} finally {
    Pop-Location
}
