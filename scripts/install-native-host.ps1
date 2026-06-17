# Install VaultPass native messaging host for Chrome and Firefox (Windows)
#
# Usage:
#   .\install-native-host.ps1 -ExtensionId <chrome-extension-id>
#
# The extension ID is shown in chrome://extensions after loading the unpacked extension.

param(
    [Parameter(Mandatory=$true)]
    [string]$ExtensionId
)

$ErrorActionPreference = "Stop"

$RepoRoot  = Split-Path $PSScriptRoot -Parent
$HostExe   = "$RepoRoot\target\release\vaultpass-native-host.exe"
$ManifestTemplate = "$PSScriptRoot\com.vaultpass.native.json"

# Build native host if not present
if (-not (Test-Path $HostExe)) {
    Write-Host "Building native host..."
    Push-Location $RepoRoot
    cargo build --release -p vaultpass-native-host
    Pop-Location
}

# Write manifest JSON to %APPDATA%\VaultPass\
$InstallDir = "$env:APPDATA\VaultPass"
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

$ManifestPath = "$InstallDir\com.vaultpass.native.json"
$ManifestContent = @"
{
  "name": "com.vaultpass.native",
  "description": "VaultPass Native Messaging Host",
  "path": "$($HostExe.Replace('\','\\'))",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://$ExtensionId/"
  ]
}
"@
Set-Content -Path $ManifestPath -Value $ManifestContent -Encoding utf8

# Register in Chrome
$ChromeKey = "HKCU:\Software\Google\Chrome\NativeMessagingHosts\com.vaultpass.native"
New-Item -Path $ChromeKey -Force | Out-Null
Set-ItemProperty -Path $ChromeKey -Name "(default)" -Value $ManifestPath

# Register in Firefox (uses a separate registry path)
$FirefoxKey = "HKCU:\Software\Mozilla\NativeMessagingHosts\com.vaultpass.native"
New-Item -Path $FirefoxKey -Force | Out-Null
Set-ItemProperty -Path $FirefoxKey -Name "(default)" -Value $ManifestPath

Write-Host ""
Write-Host "Native messaging host registered successfully." -ForegroundColor Green
Write-Host "  Host exe : $HostExe"
Write-Host "  Manifest : $ManifestPath"
Write-Host "  Chrome   : $ChromeKey"
Write-Host "  Firefox  : $FirefoxKey"
Write-Host ""
Write-Host "Reload the extension in chrome://extensions and reopen any tab to test."
