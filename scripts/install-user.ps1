[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$skillSource = Join-Path $repoRoot 'integrations/aiw-agent/skills/aiw-workspace'

if (-not (Test-Path -LiteralPath $skillSource -PathType Container)) {
    throw "AIW skill source is missing: $skillSource"
}

# --force is the documented update path. --locked keeps dependency resolution
# pinned to the source release's Cargo.lock.
& cargo install --path $repoRoot --locked --force
if ($LASTEXITCODE -ne 0) {
    throw "cargo install failed with exit code $LASTEXITCODE"
}

foreach ($skillRoot in @(
    (Join-Path $HOME '.agents/skills'),
    (Join-Path $HOME '.claude/skills')
)) {
    New-Item -ItemType Directory -Force -Path $skillRoot | Out-Null
    $destination = Join-Path $skillRoot 'aiw-workspace'
    $temporary = Join-Path $skillRoot ('.aiw-workspace.installing.' + [Guid]::NewGuid().ToString('N'))

    try {
        Copy-Item -LiteralPath $skillSource -Destination $temporary -Recurse -Force
        if (Test-Path -LiteralPath $destination) {
            $backup = "$destination.backup.$((Get-Date).ToUniversalTime().ToString('yyyyMMddHHmmss')).$PID"
            Move-Item -LiteralPath $destination -Destination $backup
            Write-Output "Backed up existing skill to $backup"
        }
        Move-Item -LiteralPath $temporary -Destination $destination
    }
    finally {
        if (Test-Path -LiteralPath $temporary) {
            Remove-Item -LiteralPath $temporary -Recurse -Force
        }
    }
}

$cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $HOME '.cargo' }
$binary = Join-Path $cargoHome 'bin/aiw.exe'
Write-Output "Installed aiw to $binary"
Write-Output 'Installed personal AIW skill for Codex and Claude Code'
Write-Output 'Enable a repository with: aiw init && aiw integrate all --enforcement strict'
