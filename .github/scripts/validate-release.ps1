param(
    [Parameter(Mandatory = $true)]
    [string] $Version
)

$ErrorActionPreference = 'Stop'

$releaseDirectory = Join-Path $PSScriptRoot '..\..\src-tauri\target\release'
$releaseDirectory = [System.IO.Path]::GetFullPath($releaseDirectory)
$bundleDirectory = Join-Path $releaseDirectory 'bundle'
$portablePath = Join-Path $releaseDirectory ("What-N-When_{0}_x64-portable.exe" -f $Version)
$validationRoot = Join-Path $env:RUNNER_TEMP ("what-n-when-release-validation-{0}" -f $Version)

function Get-ExactlyOneFile {
    param([string] $Directory, [string] $Filter, [string] $Description)
    $files = @(Get-ChildItem -Path $Directory -Filter $Filter -File -ErrorAction SilentlyContinue)
    if ($files.Count -ne 1) {
        throw "Expected exactly one $Description in '$Directory'; found $($files.Count)."
    }
    if ($files[0].Length -eq 0) {
        throw "$Description is empty: $($files[0].FullName)"
    }
    return $files[0]
}

function Test-ApplicationLaunch {
    param([string] $Executable, [string] $Description)
    if (-not (Test-Path -LiteralPath $Executable -PathType Leaf)) {
        throw "$Description executable was not found: $Executable"
    }
    $process = Start-Process -FilePath $Executable -PassThru
    try {
        Start-Sleep -Seconds 5
        $process.Refresh()
        if ($process.HasExited) {
            throw "$Description exited during the startup smoke test (exit code $($process.ExitCode))."
        }
    }
    finally {
        if (-not $process.HasExited) {
            Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
        }
    }
}

function Invoke-MsiExec {
    param([string[]] $Arguments)
    $process = Start-Process -FilePath 'msiexec.exe' -ArgumentList $Arguments -Wait -PassThru
    if ($process.ExitCode -notin @(0, 3010)) {
        throw "msiexec failed with exit code $($process.ExitCode): $($Arguments -join ' ')"
    }
}

$nsisInstaller = Get-ExactlyOneFile -Directory (Join-Path $bundleDirectory 'nsis') -Filter '*.exe' -Description 'NSIS installer'
$msiInstaller = Get-ExactlyOneFile -Directory (Join-Path $bundleDirectory 'msi') -Filter '*.msi' -Description 'MSI installer'
$msixPackage = Get-ExactlyOneFile -Directory (Join-Path $bundleDirectory 'msix') -Filter '*.msix' -Description 'MSIX package'

if (-not (Test-Path -LiteralPath $portablePath -PathType Leaf)) {
    throw "Portable executable was not found: $portablePath"
}
if ((Get-Item -LiteralPath $portablePath).Length -eq 0) {
    throw "Portable executable is empty: $portablePath"
}

$portableVersion = [System.Diagnostics.FileVersionInfo]::GetVersionInfo($portablePath).ProductVersion
if ($portableVersion -and -not $portableVersion.StartsWith($Version, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Portable executable version '$portableVersion' does not match release version '$Version'."
}

if (Test-Path -LiteralPath $validationRoot) {
    Remove-Item -LiteralPath $validationRoot -Recurse -Force
}
New-Item -Path $validationRoot -ItemType Directory -Force | Out-Null

try {
    Write-Host 'Validating MSIX package contents...'
    $makeAppx = Get-Command makeappx.exe -ErrorAction SilentlyContinue
    if (-not $makeAppx) {
        $sdkRoots = @($env:WindowsSdkDir, ${env:ProgramFiles(x86)}, $env:ProgramFiles) |
            Where-Object { $_ } |
            ForEach-Object { Join-Path $_ 'Windows Kits\10\bin' }
        $makeAppx = Get-ChildItem -Path $sdkRoots -Filter 'makeappx.exe' -File -Recurse -ErrorAction SilentlyContinue |
            Where-Object { $_.Directory.Name -eq 'x64' } |
            Sort-Object FullName -Descending |
            Select-Object -First 1
    }
    if (-not $makeAppx) {
        throw 'makeappx.exe was not found in PATH or the Windows SDK.'
    }
    $makeAppxPath = if ($makeAppx.PSObject.Properties.Name -contains 'Source') { $makeAppx.Source } else { $makeAppx.FullName }
    $msixDirectory = Join-Path $validationRoot 'msix'
    & $makeAppxPath unpack /p $msixPackage.FullName /d $msixDirectory /o
    if ($LASTEXITCODE -ne 0) {
        throw "makeappx.exe could not unpack the MSIX package (exit code $LASTEXITCODE)."
    }

    $manifestPath = Join-Path $msixDirectory 'AppxManifest.xml'
    if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
        throw 'MSIX package does not contain AppxManifest.xml.'
    }
    [xml]$manifest = Get-Content -LiteralPath $manifestPath -Raw
    $identity = $manifest.SelectSingleNode("//*[local-name()='Identity']")
    if (-not $identity -or $identity.GetAttribute('Name') -ne '56890AusnadianProductions.NotionCalendarWidget') {
        throw 'MSIX package identity does not match the expected What-N-When package identity.'
    }

    $expectedVersionParts = @($Version.Split('-')[0].Split('.'))
    while ($expectedVersionParts.Count -lt 4) { $expectedVersionParts += '0' }
    $expectedPackageVersion = $expectedVersionParts -join '.'
    if ($identity.GetAttribute('Publisher') -ne 'CN=A878E59E-93F9-435A-9DFB-177C285947A6') {
        throw 'MSIX package publisher does not match the expected package publisher.'
    }
    if ($identity.GetAttribute('Version') -ne $expectedPackageVersion) {
        throw "MSIX version '$($identity.Version)' does not match expected version '$expectedPackageVersion'."
    }

    $requiredFiles = @(
        'what-n-when.exe',
        'what-n-when-widget-provider.exe',
        'Assets\Logo.png',
        'Assets\ProviderIcon.png',
        'Assets\WidgetPreview.png'
    )
    foreach ($relativePath in $requiredFiles) {
        $path = Join-Path $msixDirectory $relativePath
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            throw "MSIX package is missing required file: $relativePath"
        }
        if ((Get-Item -LiteralPath $path).Length -eq 0) {
            throw "MSIX package contains an empty required file: $relativePath"
        }
    }

    Write-Host 'Testing portable executable startup...'
    $portableDirectory = Join-Path $validationRoot 'portable'
    New-Item -Path $portableDirectory -ItemType Directory -Force | Out-Null
    $portableCopy = Join-Path $portableDirectory 'what-n-when.exe'
    Copy-Item -LiteralPath $portablePath -Destination $portableCopy
    Test-ApplicationLaunch -Executable $portableCopy -Description 'Portable application'

    Write-Host 'Testing NSIS installation and startup...'
    $nsisInstallDirectory = Join-Path $validationRoot 'nsis-install'
    $nsisProcess = Start-Process -FilePath $nsisInstaller.FullName -ArgumentList @('/S', "/D=$nsisInstallDirectory") -Wait -PassThru
    if ($nsisProcess.ExitCode -ne 0) {
        throw "NSIS installer failed with exit code $($nsisProcess.ExitCode)."
    }
    $installedExecutable = Get-ChildItem -Path $nsisInstallDirectory -Filter 'what-n-when.exe' -File -Recurse |
        Select-Object -First 1
    if (-not $installedExecutable) {
        throw 'NSIS installation did not contain what-n-when.exe.'
    }
    try {
        Test-ApplicationLaunch -Executable $installedExecutable.FullName -Description 'NSIS-installed application'
    }
    finally {
        $uninstaller = Get-ChildItem -Path $nsisInstallDirectory -Filter 'unins*.exe' -File |
            Select-Object -First 1
        if ($uninstaller) {
            $uninstallProcess = Start-Process -FilePath $uninstaller.FullName -ArgumentList '/S' -Wait -PassThru
            if ($uninstallProcess.ExitCode -ne 0) {
                throw "NSIS uninstaller failed with exit code $($uninstallProcess.ExitCode)."
            }
        }
    }

    Write-Host 'Testing MSI installation and startup...'
    $msiInstallDirectory = Join-Path $validationRoot 'msi-install'
    New-Item -Path $msiInstallDirectory -ItemType Directory -Force | Out-Null
    Invoke-MsiExec -Arguments @('/i', $msiInstaller.FullName, '/qn', '/norestart', "INSTALLDIR=$msiInstallDirectory")
    try {
        $msiExecutable = Get-ChildItem -Path $msiInstallDirectory -Filter 'what-n-when.exe' -File -Recurse |
            Select-Object -First 1
        if (-not $msiExecutable) {
            throw 'MSI installation did not place what-n-when.exe in the requested installation directory.'
        }
        Test-ApplicationLaunch -Executable $msiExecutable.FullName -Description 'MSI-installed application'
    }
    finally {
        Invoke-MsiExec -Arguments @('/x', $msiInstaller.FullName, '/qn', '/norestart')
    }

    Write-Host ("All release package validations passed for version {0}." -f $Version)
}
finally {
    if (Test-Path -LiteralPath $validationRoot) {
        Remove-Item -LiteralPath $validationRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}
