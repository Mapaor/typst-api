<#
.SYNOPSIS
Installs Typst API for Windows.

.DESCRIPTION
Downloads the latest or specified release of Typst API from GitHub, extracts it to the local user directory, and sets up a default .env file.

.PARAMETER Version
The specific version to install (e.g., v0.2.4). If not provided, the latest release will be used.

.PARAMETER InstallDir
Custom installation directory. Defaults to "$env:USERPROFILE\.local\typst-api".

.EXAMPLE
.\install.ps1
Installs the latest version to the default directory.

.EXAMPLE
.\install.ps1 -Version "v0.2.4"
Installs version v0.2.4.

.EXAMPLE
.\install.ps1 -InstallDir "C:\typst-api"
Installs to C:\typst-api.
#>

param (
    [string]$Version,
    [string]$InstallDir = "$env:USERPROFILE\.local\typst-api"
)

$ErrorActionPreference = "Stop"
$Repo = "Mapaor/typst-api"

# --- Architecture Detection ---
$Arch = $env:PROCESSOR_ARCHITECTURE
if ($Arch -eq "AMD64") {
    $Target = "x86_64-pc-windows-msvc"
}
elseif ($Arch -eq "ARM64") {
    $Target = "aarch64-pc-windows-msvc"
}
else {
    Write-Error "Unsupported architecture: $Arch"
    exit 1
}

$AssetName = "typst-api-${Target}.zip" # Ignored (let's assume if it's .zip or .tar.gz based on github API).

# --- Determine Version ---
if ([string]::IsNullOrWhiteSpace($Version)) {
    Write-Host "Fetching latest version information..."
    try {
        $ReleaseUrl = "https://api.github.com/repos/$Repo/releases/latest"
        $Release = Invoke-RestMethod -Uri $ReleaseUrl
        $Version = $Release.tag_name
    }
    catch {
        Write-Error "Failed to fetch latest version information."
        exit 1
    }
}

Write-Host "Installing typst-api version: $Version"

# Fetch release metadata to find the exact asset name
$ReleaseData = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/tags/$Version"
$Asset = $ReleaseData.assets | Where-Object { $_.name -like "*${Target}*" } | Select-Object -First 1

if (-not $Asset) {
    Write-Error "Could not find an asset for target $Target in release $Version."
    exit 1
}

$ActualAssetName = $Asset.name
$DownloadUrl = $Asset.browser_download_url
$ChecksumUrl = "https://github.com/$Repo/releases/download/$Version/$($ActualAssetName).sha256"

# --- Directory Setup ---
Write-Host "Installation directory: $InstallDir"

if (-not (Test-Path -Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir | Out-Null
}
$FontsDir = Join-Path $InstallDir "fonts"
if (-not (Test-Path -Path $FontsDir)) {
    New-Item -ItemType Directory -Path $FontsDir | Out-Null
}

$Port = 8080
$EnvPath = Join-Path $InstallDir ".env"
if (-not (Test-Path -Path $EnvPath)) {
    function Test-PortInUse([int]$PortNumber) {
        try {
            if (Get-Command Get-NetTCPConnection -ErrorAction SilentlyContinue) {
                return $null -ne (Get-NetTCPConnection -LocalPort $PortNumber -State Listen -ErrorAction SilentlyContinue)
            }

            $Matches = netstat -ano -p TCP | Select-String -Pattern "LISTENING\s+\S+:$PortNumber\s"
            return $null -ne $Matches
        }
        catch {
            return $false
        }
    }

    do {
        $PortInput = Read-Host "Enter the server port [8080]"
        if ([string]::IsNullOrWhiteSpace($PortInput)) {
            $Port = 8080
            $ValidPort = $true
        }
        else {
            $ValidPort = [int]::TryParse($PortInput, [ref]$Port)
        }
        if (-not $ValidPort -or $Port -lt 1 -or $Port -gt 65535) {
            Write-Host "Please enter a valid port between 1 and 65535."
        }
        elseif (Test-PortInUse $Port) {
            Write-Host "Port $Port is already in use. Please choose another port."
            $ValidPort = $false
        }
    } while (-not $ValidPort -or $Port -lt 1 -or $Port -gt 65535)
}

# --- Download & Verify ---
$TempDir = Join-Path -Path ([System.IO.Path]::GetTempPath()) -ChildPath ([guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $TempDir | Out-Null
$DownloadPath = Join-Path $TempDir $ActualAssetName
$ChecksumPath = Join-Path $TempDir "checksum.sha256"

try {
    Write-Host "Downloading $ActualAssetName..."
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $DownloadPath

    Write-Host "Downloading checksum..."
    try {
        Invoke-WebRequest -Uri $ChecksumUrl -OutFile $ChecksumPath
        Write-Host "Verifying checksum..."
        $ExpectedHash = (Get-Content $ChecksumPath).Split(' ')[0]
        $ActualHash = (Get-FileHash -Path $DownloadPath -Algorithm SHA256).Hash.ToLower()
        
        if ($ExpectedHash -and ($ExpectedHash.ToLower() -ne $ActualHash)) {
            Write-Error "Checksum verification failed! Expected: $ExpectedHash, Actual: $ActualHash"
            exit 1
        }
        Write-Host "Checksum verified."
    }
    catch {
        Write-Host "Warning: Checksum file not found or verification skipped."
    }

    # --- Extraction ---
    Write-Host "Extracting archive..."
    if ($ActualAssetName -match "\.zip$") {
        Expand-Archive -Path $DownloadPath -DestinationPath $TempDir -Force
    }
    elseif ($ActualAssetName -match "\.tar\.gz$") {
        # tar is available in Windows 10+
        tar -xzf $DownloadPath -C $TempDir
    }
    else {
        Write-Error "Unknown archive format: $ActualAssetName"
        exit 1
    }

    $ExtractDir = Join-Path $TempDir ($ActualAssetName -replace "\.(zip|tar\.gz)$", "")
    if (-not (Test-Path -Path $ExtractDir)) {
        $ExtractDir = $TempDir
    }

    # Copy files
    $ExePath = Join-Path $ExtractDir "typst-api.exe"
    if (-not (Test-Path -Path $ExePath)) {
        $ExePath = Join-Path $ExtractDir "typst-api"
    }

    if (Test-Path -Path $ExePath) {
        Copy-Item -Path $ExePath -Destination $InstallDir -Force
    }

    $ExtractFontsDir = Join-Path $ExtractDir "assets\fonts"
    if (Test-Path -Path $ExtractFontsDir) {
        Copy-Item -Path "$ExtractFontsDir\*" -Destination $FontsDir -Recurse -Force
    }

    # --- Configuration Setup ---
    if (-not (Test-Path -Path $EnvPath)) {
        $ExampleEnv = Join-Path $ExtractDir ".env.example"
        if (Test-Path -Path $ExampleEnv) {
            Write-Host "Generating .env file from .env.example..."
            (Get-Content $ExampleEnv) -replace '^PORT=.*', "PORT=$Port" | Set-Content $EnvPath
        }
        else {
            Write-Host "Creating basic .env file..."
            $EnvContent = @"
PORT=$Port
MAX_CONCURRENT_COMPILATIONS=10
# AUTH_TOKEN=my-secret-token
# ADMIN_TOKEN=my-admin-secret-token
"@
            Set-Content -Path $EnvPath -Value $EnvContent
        }
    }
    else {
        Write-Host ".env file already exists. Skipping generation."
    }

    # --- Health Check ---
    Write-Host "Running health check..."
    $InstalledExe = Join-Path $InstallDir "typst-api.exe"
    if (Test-Path -Path $InstalledExe) {
        Write-Host "Installation completed successfully! 🎉"
        Write-Host "You can run the server using:"
        Write-Host "  cd `"$InstallDir`" ; .\typst-api.exe"
    }
    else {
        Write-Host "Warning: Executable check failed. It might require additional libraries."
    }

}
finally {
    if (Test-Path -Path $TempDir) {
        Remove-Item -Path $TempDir -Recurse -Force
    }
}
