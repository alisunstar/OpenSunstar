param(
  [Parameter(Mandatory = $true)]
  [string]$Version
)

$ErrorActionPreference = "Stop"

$normalized = $Version
if ($normalized.StartsWith("v")) {
  $normalized = $normalized.Substring(1)
}
$tag = "v$normalized"

if ($normalized -notmatch '^\d+\.\d+\.\d+$') {
  throw "Version must be SemVer (x.y.z). Got: $Version"
}

if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
  throw "git is required"
}

if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
  throw "node is required"
}

$status = git status --porcelain
if ($status) {
  throw "Git working tree is not clean. Commit or stash changes first."
}

$pkgVersion = node -e "process.stdout.write(require('./package.json').version)"
$tauriVersion = node -e "process.stdout.write(require('./src-tauri/tauri.conf.json').version)"

if ($pkgVersion -ne $normalized -or $tauriVersion -ne $normalized) {
  throw @"
Version mismatch.
  target:            $normalized
  package.json:      $pkgVersion
  src-tauri config:  $tauriVersion
Please update both files before tagging.
"@
}

git rev-parse $tag *> $null
if ($LASTEXITCODE -eq 0) {
  throw "Local tag already exists: $tag"
}

git ls-remote --exit-code --tags origin "refs/tags/$tag" *> $null
if ($LASTEXITCODE -eq 0) {
  throw "Remote tag already exists: $tag"
}

Write-Host "Creating tag: $tag"
git tag -a $tag -m "Release $tag"

Write-Host "Pushing branch HEAD to origin"
git push origin HEAD

Write-Host "Pushing tag to origin"
git push origin $tag

Write-Host "Done. GitHub Actions release workflow should start for $tag."
