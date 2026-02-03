param(
  [Parameter(Mandatory = $true)]
  [string]$Version,

  [string]$Branch = "v0.5",

  [string]$Remote = "origin",

  [switch]$Yes
)

$ErrorActionPreference = "Stop"

function Get-RepoRoot() {
  return (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
}

function Exec([string]$Cmd) {
  Write-Host ("`n> " + $Cmd)
  iex $Cmd
  if ($LASTEXITCODE -ne 0) {
    throw "Command failed with exit code ${LASTEXITCODE}: $Cmd"
  }
}

function Confirm-Step([string]$Message) {
  if ($Yes) { return }
  $ans = Read-Host $Message
  if ($ans -notin @("y", "Y", "yes", "YES")) {
    throw "Canceled."
  }
}

$RepoRoot = Get-RepoRoot
Set-Location $RepoRoot

# Normalize version + tag.
$v = $Version.Trim()
if ($v.StartsWith("v")) { $v = $v.Substring(1) }
if ($v -notmatch "^\d+\.\d+\.\d+$") {
  throw "Invalid -Version '$Version'. Expected like 0.5.7"
}
$Tag = "v$v"

Exec "git rev-parse --is-inside-work-tree | Out-Null"

# Make sure we are on the intended branch (avoids tagging the wrong commit).
$curBranch = (& git branch --show-current | Out-String).Trim()
if ([string]::IsNullOrWhiteSpace($curBranch)) {
  throw "Git did not report a current branch (detached HEAD?). Switch to '$Branch': git checkout $Branch"
}
if ($curBranch -ne $Branch) {
  throw "You are on branch '$curBranch'. Switch to '$Branch' first: git checkout $Branch"
}

# Refuse to overwrite an existing tag (local or remote).
$localTag = (& git tag --list $Tag | Out-String).Trim()
if ($localTag -eq $Tag) {
  throw "Tag already exists locally: $Tag. Delete it or choose a new version."
}
$remoteTag = (& git ls-remote --tags $Remote $Tag | Out-String)
if (-not [string]::IsNullOrWhiteSpace($remoteTag)) {
  throw "Tag already exists on ${Remote}: ${Tag}. Delete it or choose a new version."
}

Confirm-Step "About to bump versions to $v, commit, push '$Branch', and push tag '$Tag'. Type 'y' to continue"

# 1) Bump version files.
Exec ".\\scripts\\bump-version.ps1 -Version $v"

# 2) Commit everything currently changed (including the version bump).
Exec "git add -A"
$pending = (git status --porcelain=v1) | Out-String
if ([string]::IsNullOrWhiteSpace($pending)) {
  throw "Nothing to commit after bump-version. Aborting (no release commit created)."
}
Exec "git commit -m ""chore(release): $Tag"""

# 3) Push branch, then create annotated tag on that exact commit, then push tag.
Exec "git push $Remote $Branch"
Exec "git tag -a $Tag -m ""$Tag"""
Exec "git push $Remote $Tag"

Write-Host "`nDone. GitHub Actions should start from tag '$Tag'."
