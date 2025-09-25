[CmdletBinding()]
param(
    [string]$Version = $env:OBSYNCGIT_VERSION,
    [string]$InstallDir = $env:OBSYNCGIT_INSTALL_DIR
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repo = 'GezzyDax/ObsyncGit'
$project = 'obsyncgit'
$target = 'x86_64-pc-windows-msvc'
$pathUpdated = $false

if (-not $Version) {
    $Version = 'latest'
}
if (-not $InstallDir) {
    $InstallDir = Join-Path (Join-Path $env:LOCALAPPDATA 'ObsyncGit') 'bin'
}

$arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
if ($arch -ne [System.Runtime.InteropServices.Architecture]::X64) {
    throw "Unsupported Windows architecture: $arch"
}

if ($Version -eq 'latest') {
    $downloadUrl = "https://github.com/$repo/releases/latest/download/$project-$target.zip"
} else {
    if ($Version -notmatch '^v') {
        $Version = "v$Version"
    }
    $downloadUrl = "https://github.com/$repo/releases/download/$Version/$project-$target.zip"
}

$temporaryDir = New-Item -ItemType Directory -Path ([System.IO.Path]::Combine([System.IO.Path]::GetTempPath(), [System.Guid]::NewGuid().ToString()))
try {
    $archivePath = Join-Path $temporaryDir.FullName "$project-$target.zip"
    Write-Host "Downloading $downloadUrl"
    Invoke-WebRequest -Uri $downloadUrl -OutFile $archivePath -UseBasicParsing

    $extractDir = Join-Path $temporaryDir.FullName 'extract'
    Expand-Archive -LiteralPath $archivePath -DestinationPath $extractDir -Force
    $binaryPath = Join-Path $extractDir "$project.exe"
    if (-not (Test-Path $binaryPath)) {
        throw 'Failed to extract executable from archive.'
    }

    $null = New-Item -ItemType Directory -Path $InstallDir -Force
    $destination = Join-Path $InstallDir "$project.exe"
    Copy-Item -LiteralPath $binaryPath -Destination $destination -Force

    $userPath = [Environment]::GetEnvironmentVariable('PATH', 'User')
    $pathEntries = @()
    if ($userPath) {
        $pathEntries = $userPath.Split(';') | Where-Object { $_ }
    }
    if (-not ($pathEntries -contains $InstallDir)) {
        $newPath = if ($userPath) { "$userPath;$InstallDir" } else { $InstallDir }
        [Environment]::SetEnvironmentVariable('PATH', $newPath, 'User')
        $pathUpdated = $true
    }

    $versionOutput = ''
    try {
        $versionOutput = & $destination --version
    } catch {
        $versionOutput = ''
    }

    Write-Host "Installed $project to $destination"
    if ($versionOutput) {
        Write-Host $versionOutput
    }
    if ($pathUpdated) {
        Write-Host 'PATH updated. Restart your terminal session to use the new command.'
    } else {
        Write-Host 'Ensure your PATH contains the install directory to use the command everywhere.'
    }
}
finally {
    if (Test-Path $temporaryDir.FullName) {
        Remove-Item -LiteralPath $temporaryDir.FullName -Recurse -Force
    }
}
