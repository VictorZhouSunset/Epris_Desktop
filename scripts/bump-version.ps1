param(
  [Parameter(Mandatory = $true)]
  [string]$Version
)

$ErrorActionPreference = "Stop"

function Get-RepoRoot() {
  # Do not rely on the process current directory (can be C:\Windows\System32).
  # Anchor relative paths to the script location.
  return (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
}

function Update-JsonVersion([string]$Path, [string]$Version) {
  if (-not (Test-Path $Path)) {
    throw "File not found: $Path"
  }

  $json = Get-Content $Path -Raw | ConvertFrom-Json
  $json.version = $Version

  $out = $json | ConvertTo-Json -Depth 100
  # Keep UTF-8 without BOM for cross-platform friendliness
  [System.IO.File]::WriteAllText($Path, $out + "`n", (New-Object System.Text.UTF8Encoding($false)))
}

$RepoRoot = Get-RepoRoot
$AppPackageJson = Join-Path $RepoRoot "apps/epris-tauri/package.json"
$TauriConfJson = Join-Path $RepoRoot "apps/epris-tauri/src-tauri/tauri.conf.json"

Update-JsonVersion $AppPackageJson $Version
Update-JsonVersion $TauriConfJson $Version

Write-Host ("Repo root: " + $RepoRoot)
Write-Host "Updated versions to ${Version}:"
Write-Host ("- apps/epris-tauri/package.json -> " + ((Get-Content $AppPackageJson -Raw | ConvertFrom-Json).version))
Write-Host ("- apps/epris-tauri/src-tauri/tauri.conf.json -> " + ((Get-Content $TauriConfJson -Raw | ConvertFrom-Json).version))
