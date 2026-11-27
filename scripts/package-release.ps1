param(
    [string]$Version = "1.0.0"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$artifactDir = Join-Path $root "artifacts"
$folderName = "RustyWatch-$Version-win-x64"
$publishDir = Join-Path $artifactDir $folderName
$zipPath = Join-Path $artifactDir "$folderName.zip"

Write-Host "== Rusty Watch release $Version =="

# Jedno źródło wersji: workspace Cargo.toml.
$cargo = Get-Content (Join-Path $root "Cargo.toml") -Raw
if ($cargo -notmatch "(?m)^version\s*=\s*""$([regex]::Escape($Version))""") {
    throw "Cargo.toml workspace version is not $Version"
}

if (Test-Path $publishDir) { Remove-Item $publishDir -Recurse -Force }
New-Item -ItemType Directory -Path $publishDir -Force | Out-Null

Push-Location $root
try {
    Write-Host "Building rusty-ffi (release)..."
    cargo build --release -p rusty-ffi
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

    Write-Host "Publishing self-contained WinUI app..."
    dotnet publish (Join-Path $root "native\RustyWatch\RustyWatch.csproj") `
        -c Release -r win-x64 --self-contained true `
        -p:Version=$Version -o $publishDir
    if ($LASTEXITCODE -ne 0) { throw "dotnet publish failed" }
}
finally {
    Pop-Location
}

# Weryfikacja kompletności paczki.
$required = @(
    "RustyWatch.exe", "RustyWatch.dll", "rusty_ffi.dll",
    "App.xbf", "MainWindow.xbf", "VcRedistWindow.xbf", "RustyWatch.pri",
    "Localization\en.json", "Localization\pl.json",
    "Assets\Fonts\NotoSans-Regular.ttf", "Assets\Icons\click-zone.png"
)
foreach ($relative in $required) {
    if (-not (Test-Path (Join-Path $publishDir $relative))) {
        throw "Missing published file: $relative"
    }
}

Get-ChildItem $publishDir -Filter "*.pdb" -Recurse -File | Remove-Item -Force

Copy-Item (Join-Path $root "LICENSE") (Join-Path $publishDir "LICENSE") -Force
Copy-Item (Join-Path $root "THIRD_PARTY_LICENSES.md") (Join-Path $publishDir "THIRD_PARTY_LICENSES.md") -Force
Set-Content (Join-Path $publishDir "VERSION.txt") $Version -NoNewline -Encoding UTF8

$readme = @"
Rusty Watch $Version (Windows x64)
==================================

Quick start
-----------
1. Extract the whole archive into a writable folder (Documents or Desktop).
   Do NOT run it from Program Files: the app writes FontCache next to the exe.
2. Install Microsoft Visual C++ Redistributable 2015-2022 (x64) if asked:
   https://aka.ms/vs/17/release/vc_redist.x64.exe
   The app checks for it at startup and shows a link if it is missing.
3. Run RustyWatch.exe. Windows SmartScreen may warn because the build is not
   signed; choose "More info" -> "Run anyway" if you trust this release.

Requirements
------------
- Windows 10 20H1 (build 19041) or newer, x64
- VC++ Redistributable 2015-2022 (x64)
- No .NET installation is needed: the runtime is bundled.
- Optional: Zepp OS Simulator with the Active 2 v1.1.0 device image.

User data is created in %APPDATA%\RustyWatch. Uninstall = delete this folder
and optionally the appdata directory.

Source and licence: https://github.com/nawszym/rusty-watch
"@
Set-Content (Join-Path $publishDir "README.txt") $readme -Encoding UTF8

$hashes = Get-ChildItem $publishDir -File -Recurse |
    Where-Object { $_.Name -ne "SHA256SUMS.txt" } |
    ForEach-Object {
        $relative = $_.FullName.Substring($publishDir.Length + 1)
        "{0}  {1}" -f (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant(), $relative
    } | Sort-Object
Set-Content (Join-Path $publishDir "SHA256SUMS.txt") $hashes -Encoding UTF8

if (Test-Path $zipPath) { Remove-Item $zipPath -Force }
Compress-Archive -Path $publishDir -DestinationPath $zipPath -CompressionLevel Optimal

Write-Host "Release package: $zipPath"
Write-Host ("Size: {0:N1} MB" -f ((Get-Item $zipPath).Length / 1MB))
