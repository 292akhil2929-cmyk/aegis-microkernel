param([string]$Qemu = 'qemu-system-aarch64')
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$kernel = Join-Path $repoRoot 'target\aarch64-unknown-none-softfloat\release\aegis-kernel'
if (-not (Test-Path -LiteralPath $kernel)) { & (Join-Path $PSScriptRoot 'build.ps1') }
& $Qemu -M virt,gic-version=2 -cpu cortex-a72 -m 256M -smp 1 -nographic -monitor none -serial stdio -kernel $kernel

