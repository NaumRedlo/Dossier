param([string]$TargetDir)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
if (!$TargetDir) { $TargetDir = Join-Path $root 'target/windows-test' }
$TargetDir = [IO.Path]::GetFullPath($TargetDir)
$target = 'x86_64-pc-windows-msvc'
$release = Join-Path $TargetDir "$target/release"
$oldWitness = $env:DOSSIER_WITNESS_EXE
$oldOverlay = $env:DOSSIER_WITNESS_OVERLAY_EXE
Push-Location $root
try {
    & cargo build --locked --release --target $target --target-dir $TargetDir -p dossier-witness -p dossier-witness-overlay
    if ($LASTEXITCODE) { throw 'Witness build failed' }
    $env:DOSSIER_WITNESS_EXE = (Get-Item (Join-Path $release 'witness.exe')).FullName
    $env:DOSSIER_WITNESS_OVERLAY_EXE = (Get-Item (Join-Path $release 'witness-overlay.exe')).FullName
    & cargo build --locked --release --manifest-path native/Cargo.toml --target $target --target-dir $TargetDir
    if ($LASTEXITCODE) { throw 'Dossier build failed' }
    Write-Output (Join-Path $release 'dossier.exe')
} finally {
    $env:DOSSIER_WITNESS_EXE = $oldWitness
    $env:DOSSIER_WITNESS_OVERLAY_EXE = $oldOverlay
    Pop-Location
}
