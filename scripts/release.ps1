<#
Publishes a version of Prospector (docs/RELEASE.md, "Each version").
Run it yourself, from the project folder, in PowerShell:

    .\scripts\release.ps1                 # checks, commit, push, tag
    .\scripts\release.ps1 -SkipChecks     # same, without the local checks

It reads the version in src-tauri/tauri.conf.json, checks that Cargo.toml
and package.json say the same and that CHANGELOG.md has a section for it,
runs the checks of the release workflow, then commits everything, pushes
main and the tag vX.Y.Z. GitHub Actions (release.yml) then builds the
installers and the portable ZIP and makes a draft release: open it,
check it, and click "Publish release". The download page uses it at once.
#>
param(
    [switch]$SkipChecks,
    [string]$Message
)

$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)

function Step($text) { Write-Host "`n== $text" -ForegroundColor Cyan }
function Fail($text) { Write-Host "`n$text" -ForegroundColor Red; exit 1 }
function Run($exe, [string[]]$arguments) {
    # git writes its progress on stderr: only the exit code counts.
    $ErrorActionPreference = 'Continue'
    & $exe @arguments
    if ($LASTEXITCODE -ne 0) { Fail "Failed: $exe $($arguments -join ' ')" }
}

Step 'Version'
$version = (Get-Content src-tauri/tauri.conf.json -Raw | ConvertFrom-Json).version
$cargo = (Select-String -Path Cargo.toml -Pattern '^version = "(.+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
$node = (Get-Content package.json -Raw | ConvertFrom-Json).version
if ($cargo -ne $version -or $node -ne $version) {
    Fail "Versions differ: tauri.conf.json $version, Cargo.toml $cargo, package.json $node"
}
if (-not (Select-String -Path CHANGELOG.md -Pattern "^## $([regex]::Escape($version)) " -Quiet)) {
    Fail "CHANGELOG.md has no '## $version' section"
}
$tag = "v$version"
Write-Host "Version $version, tag $tag"

# git.exe itself: a `git` function or alias of the PowerShell profile (with
# a parameter such as -Args) would take `-A` or `-m` for its own (BUG-080).
$git = (Get-Command git -CommandType Application -ErrorAction Stop | Select-Object -First 1).Source

$branch = (& $git rev-parse --abbrev-ref HEAD).Trim()
if ($branch -ne 'main') { Fail "On branch '$branch': switch to main first" }
Run $git @('fetch', '--tags', '--quiet')
if (& $git tag --list $tag) { Fail "The tag $tag already exists: raise the version first" }

# A check prints only its last lines; the whole output goes to target/tmp.
function Check($name, $exe, [string[]]$arguments) {
    $log = "target/tmp/release-$name.log"
    New-Item -ItemType Directory -Force target/tmp | Out-Null
    Write-Host "  $name... " -NoNewline
    # Windows PowerShell turns any redirected stderr line (cargo's
    # "Compiling ...") into an error under 'Stop': only the exit code counts.
    $ErrorActionPreference = 'Continue'
    & $exe @arguments *> $log
    if ($LASTEXITCODE -ne 0) {
        Get-Content $log -Tail 30
        Fail "Failed: $name (whole output in $log)"
    }
    Write-Host 'ok' -ForegroundColor Green
}

if (-not $SkipChecks) {
    Step 'Checks (a few minutes)'
    Check 'svelte-check' pnpm @('check')
    Check 'i18n' pnpm @('i18n:check')
    Check 'clippy' cargo @('clippy', '--workspace', '--all-targets', '--', '-D', 'warnings')
    Check 'tests' cargo @('test', '-p', 'prospector-core', '-p', 'prospector-cli')
}

Step 'Commit and push'
Run $git @('add', '--all')
& $git status --short
# Exit code 1: something is staged.
& $git diff --cached --quiet
if ($LASTEXITCODE -eq 0) {
    Write-Host 'Nothing new to commit.'
} else {
    if (-not $Message) { $Message = "Prospector $version" }
    Run $git @('commit', '-m', $Message)
}
Run $git @('push', 'origin', 'main')

Step "Tag $tag"
Run $git @('tag', '-a', $tag, '-m', "Prospector $version")
Run $git @('push', 'origin', $tag)

$repo = (Get-Content site/config.js -Raw | Select-String -Pattern "PROSPECTOR_REPO\s*=\s*'([^']+)'").Matches[0].Groups[1].Value
Write-Host "`nDone. GitHub Actions is building $tag (about 20 to 30 minutes):" -ForegroundColor Green
Write-Host "  https://github.com/$repo/actions"
Write-Host "Then open the draft, check it and click 'Publish release':"
Write-Host "  https://github.com/$repo/releases"
