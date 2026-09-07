[CmdletBinding()]
param(
    [string]$ProjectRoot = (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)),
    [switch]$SkipNativeDependencies,
    [switch]$SkipTests
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$VcpkgToolCommit = "0ac8df3b98e3afcd8bf075fa74a6bd2c32613345"
$SciterCommit = "f33df075d9eb2f8d252cb88f1b2c8096e56197ed"
$SciterSha256 = "4D97528E157C55EF1FABE9E37A9697116AB66660D7DA6163F90A3A7ABF80DD56"
$ClientVersion = "1.4.9"

$ProjectRoot = [IO.Path]::GetFullPath($ProjectRoot)
$RustDeskDir = $ProjectRoot
$VcpkgRoot = Join-Path $ProjectRoot ".tools\vcpkg"
$LibClangRoot = Join-Path $ProjectRoot ".tools\libclang-python"
$LibClangPath = Join-Path $LibClangRoot "clang\native"
$CargoTargetDir = Join-Path $ProjectRoot ".cache\rustdesk-target"
$PythonPackages = Join-Path $ProjectRoot ".cache\python-packages"
$PackageRoot = Join-Path $ProjectRoot ".cache\relaisdesk-windows-package"
$OutputDir = Join-Path $ProjectRoot "bin\rustdesk-fork"
$OutputPath = Join-Path $OutputDir "RelaisDesk-RustDesk-$ClientVersion-windows-x64.exe"
$ManifestDir = $PSScriptRoot
$ManifestPath = Join-Path $ManifestDir "vcpkg.json"
$NasmSelector = Join-Path $ManifestDir "nasm-2.16.03.cmake"

function Invoke-NativeChecked {
    param(
        [Parameter(Mandatory)] [string]$FilePath,
        [string[]]$Arguments = @()
    )
    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "Échec ($LASTEXITCODE) : $FilePath $($Arguments -join ' ')"
    }
}

function Invoke-InDirectory {
    param(
        [Parameter(Mandatory)] [string]$Directory,
        [Parameter(Mandatory)] [string]$FilePath,
        [string[]]$Arguments = @()
    )
    Push-Location $Directory
    try {
        Invoke-NativeChecked -FilePath $FilePath -Arguments $Arguments
    } finally {
        Pop-Location
    }
}

foreach ($requiredPath in @($RustDeskDir, $ManifestPath, $NasmSelector)) {
    if (-not (Test-Path -LiteralPath $requiredPath)) {
        throw "Chemin requis absent : $requiredPath"
    }
}

if (-not $SkipNativeDependencies) {
    if (-not (Test-Path -LiteralPath (Join-Path $VcpkgRoot ".git"))) {
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $VcpkgRoot) | Out-Null
        Invoke-NativeChecked -FilePath "git" -Arguments @(
            "clone", "--filter=blob:none", "https://github.com/microsoft/vcpkg.git", $VcpkgRoot
        )
    }

    Invoke-InDirectory -Directory $VcpkgRoot -FilePath "git" -Arguments @("checkout", $VcpkgToolCommit)
    Invoke-NativeChecked -FilePath (Join-Path $VcpkgRoot "bootstrap-vcpkg.bat") -Arguments @("-disableMetrics")

    $VcpkgNasmSelector = Join-Path $VcpkgRoot "scripts\cmake\vcpkg_find_acquire_program(NASM).cmake"
    Copy-Item -LiteralPath $NasmSelector -Destination $VcpkgNasmSelector -Force

    $GitPerlRoot = "C:\Program Files\Git\usr"
    $PerlParent = Join-Path $VcpkgRoot "downloads\tools\perl\5.42.2.1"
    $PerlLink = Join-Path $PerlParent "perl"
    if ((Test-Path -LiteralPath $GitPerlRoot) -and -not (Test-Path -LiteralPath $PerlLink)) {
        New-Item -ItemType Directory -Force -Path $PerlParent | Out-Null
        New-Item -ItemType Junction -Path $PerlLink -Target $GitPerlRoot | Out-Null
    }

    if (-not (Test-Path -LiteralPath (Join-Path $LibClangPath "libclang.dll"))) {
        New-Item -ItemType Directory -Force -Path $LibClangRoot | Out-Null
        Invoke-NativeChecked -FilePath "python" -Arguments @(
            "-m", "pip", "install", "--disable-pip-version-check", "--target", $LibClangRoot,
            "libclang==18.1.1"
        )
    }

    Invoke-NativeChecked -FilePath (Join-Path $VcpkgRoot "vcpkg.exe") -Arguments @(
        "install",
        "--triplet=x64-windows-static",
        "--x-manifest-root=$ManifestDir",
        "--x-install-root=$(Join-Path $VcpkgRoot 'installed')",
        "--clean-after-build"
    )
}

foreach ($requiredPath in @(
    (Join-Path $VcpkgRoot "installed\x64-windows-static\lib"),
    (Join-Path $LibClangPath "libclang.dll")
)) {
    if (-not (Test-Path -LiteralPath $requiredPath)) {
        throw "Dépendance native absente : $requiredPath"
    }
}

$env:VCPKG_ROOT = $VcpkgRoot
$env:LIBCLANG_PATH = $LibClangPath
$env:CARGO_TARGET_DIR = $CargoTargetDir

if (-not $SkipTests) {
    Invoke-InDirectory -Directory $RustDeskDir -FilePath "cargo" -Arguments @("test", "--locked", "--lib")
}

Invoke-InDirectory -Directory $RustDeskDir -FilePath "python" -Arguments @("res\inline-sciter.py")
Invoke-InDirectory -Directory $RustDeskDir -FilePath "cargo" -Arguments @(
    "build", "--locked", "--release", "--features", "inline"
)
Invoke-InDirectory -Directory (Join-Path $RustDeskDir "libs\virtual_display\dylib") `
    -FilePath "cargo" -Arguments @("build", "--locked", "--release")

New-Item -ItemType Directory -Force -Path $PackageRoot, $OutputDir | Out-Null
$SciterPath = Join-Path $PackageRoot "sciter.dll"
if (-not (Test-Path -LiteralPath $SciterPath)) {
    $SciterUrl = "https://raw.githubusercontent.com/c-smile/sciter-sdk/$SciterCommit/bin.win/x64/sciter.dll"
    Invoke-WebRequest -Uri $SciterUrl -OutFile $SciterPath
}
$ActualSciterSha256 = (Get-FileHash -LiteralPath $SciterPath -Algorithm SHA256).Hash
if ($ActualSciterSha256 -ne $SciterSha256) {
    throw "Empreinte Sciter invalide : $ActualSciterSha256"
}

$PayloadDir = Join-Path $PackageRoot ("payload-{0}-{1}" -f $PID, [DateTime]::UtcNow.Ticks)
New-Item -ItemType Directory -Path $PayloadDir | Out-Null
Copy-Item -LiteralPath (Join-Path $CargoTargetDir "release\rustdesk.exe") `
    -Destination (Join-Path $PayloadDir "rustdesk.exe")
Copy-Item -LiteralPath (Join-Path $CargoTargetDir "release\dylib_virtual_display.dll") `
    -Destination (Join-Path $PayloadDir "dylib_virtual_display.dll")
Copy-Item -LiteralPath $SciterPath -Destination (Join-Path $PayloadDir "sciter.dll")

if (-not (Test-Path -LiteralPath (Join-Path $PythonPackages "brotli.py")) -and
    -not (Test-Path -LiteralPath (Join-Path $PythonPackages "_brotli.pyd"))) {
    New-Item -ItemType Directory -Force -Path $PythonPackages | Out-Null
    Invoke-NativeChecked -FilePath "python" -Arguments @(
        "-m", "pip", "install", "--disable-pip-version-check", "--target", $PythonPackages,
        "Brotli==1.2.0"
    )
}
$env:PYTHONPATH = $PythonPackages

$PortableDir = Join-Path $RustDeskDir "libs\portable"
Invoke-NativeChecked -FilePath "python" -Arguments @(
    (Join-Path $PortableDir "generate.py"),
    "-f", $PayloadDir,
    "-o", $PortableDir,
    "-e", (Join-Path $PayloadDir "rustdesk.exe"),
    "-l", "11"
)

$PackerPath = Join-Path $CargoTargetDir "release\rustdesk-portable-packer.exe"
if (-not (Test-Path -LiteralPath $PackerPath)) {
    throw "Packer absent après compilation : $PackerPath"
}
Copy-Item -LiteralPath $PackerPath -Destination $OutputPath -Force
$ForkSha256 = (Get-FileHash -LiteralPath $OutputPath -Algorithm SHA256).Hash.ToLowerInvariant()

Write-Host "Fork Windows : $OutputPath"
Write-Host "SHA-256      : $ForkSha256"
Write-Host "Note          : artefacts sans Authenticode ; annoncer cet état et signer le manifeste Ed25519 avant distribution."
