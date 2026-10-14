$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repoRoot
try {
    & cargo run -p rust-gta-3 --profile profiling --features profiling-toggle
    if ($LASTEXITCODE -ne 0) {
        throw "Profiling game exited with code $LASTEXITCODE."
    }
}
finally {
    Pop-Location
}
