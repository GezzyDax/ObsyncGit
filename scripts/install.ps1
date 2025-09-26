[CmdletBinding()]
param(
    [string]$Version = $env:OBSYNCGIT_VERSION,
    [string]$InstallDir = $env:OBSYNCGIT_INSTALL_DIR
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repo = 'GezzyDax/ObsyncGit'
$project = 'obsyncgit'
$pathUpdated = $false

function Register-ObsyncGitScheduledTask {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Executable
    )

    if (-not (Test-Path $Executable)) {
        throw "Executable '$Executable' was not found and cannot be scheduled."
    }

    $taskName = 'ObsyncGit'
    try {
        if (Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue) {
            Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction SilentlyContinue
        }
    } catch {
        Write-Verbose "No existing scheduled task named $taskName."
    }

    $action = New-ScheduledTaskAction -Execute $Executable -Argument 'run'
    $trigger = New-ScheduledTaskTrigger -AtLogOn
    $settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit (New-TimeSpan -Minutes 5)
    Register-ScheduledTask -TaskName $taskName -TaskPath '\' -Description 'Automatically start ObsyncGit daemon at logon' -Action $action -Trigger $trigger -Settings $settings -User $env:USERNAME
}

$arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
switch ($arch) {
    ([System.Runtime.InteropServices.Architecture]::X64) { $assetName = 'obsyncgit-windows-x86_64.zip' }
    ([System.Runtime.InteropServices.Architecture]::Arm64) { $assetName = 'obsyncgit-windows-arm64.zip' }
    default { throw "Unsupported Windows architecture: $arch" }
}

if (-not $Version) {
    $Version = 'latest'
}
if (-not $InstallDir) {
    $InstallDir = Join-Path (Join-Path $env:LOCALAPPDATA 'ObsyncGit') 'bin'
}

if ($Version -eq 'latest') {
    $downloadUrl = "https://github.com/$repo/releases/latest/download/$assetName"
} else {
    if ($Version -notmatch '^v') {
        $Version = "v$Version"
    }
    $downloadUrl = "https://github.com/$repo/releases/download/$Version/$assetName"
}

$temporaryDir = New-Item -ItemType Directory -Path ([System.IO.Path]::Combine([System.IO.Path]::GetTempPath(), [System.Guid]::NewGuid().ToString()))
try {
    $archivePath = Join-Path $temporaryDir.FullName $assetName
    Write-Host "Downloading $downloadUrl"
    Invoke-WebRequest -Uri $downloadUrl -OutFile $archivePath -UseBasicParsing

    $extractDir = Join-Path $temporaryDir.FullName 'extract'
    Expand-Archive -LiteralPath $archivePath -DestinationPath $extractDir -Force
    $null = New-Item -ItemType Directory -Path $InstallDir -Force

    $installed = @()
    Get-ChildItem -Path $extractDir -Filter "obsyncgit*.exe" | ForEach-Object {
        $target = Join-Path $InstallDir $_.Name
        Copy-Item -LiteralPath $_.FullName -Destination $target -Force
        $installed += $target
        Write-Host "Installed $(Split-Path -Leaf $target) to $target"
    }

    if ($installed.Count -eq 0) {
        throw 'No obsyncgit executables were found in the archive.'
    }

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

    $daemonPath = Join-Path $InstallDir "$project.exe"
    if (Test-Path $daemonPath) {
        $versionOutput = ''
        try {
            $versionOutput = & $daemonPath --version
        } catch {
            $versionOutput = ''
        }
        if ($versionOutput) {
            Write-Host $versionOutput
        }
    }
    if ($pathUpdated) {
        Write-Host 'PATH updated. Restart your terminal session to use the new command.'
    } else {
        Write-Host 'Ensure your PATH contains the install directory to use the command everywhere.'
    }
    try {
        Register-ObsyncGitScheduledTask -Executable $daemonPath
    } catch {
        Write-Warning "Failed to configure scheduled task: $($_.Exception.Message)"
    }
    Write-Host 'Autostart configured via Windows Task Scheduler (ObsyncGit).'    
}
finally {
    if (Test-Path $temporaryDir.FullName) {
        Remove-Item -LiteralPath $temporaryDir.FullName -Recurse -Force
    }
}
