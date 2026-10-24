param(
    [ValidateNotNullOrEmpty()]
    [string] $CertificateThumbprint,
    [string] $PackagePath,
    [string] $TimestampServer = 'http://timestamp.digicert.com'
)

$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$packageDirectory = Join-Path $repositoryRoot 'src-tauri\target\release\bundle\msix'

if (-not $CertificateThumbprint) {
    if ($env:WNW_MSIX_CERTIFICATE_THUMBPRINT) {
        $CertificateThumbprint = $env:WNW_MSIX_CERTIFICATE_THUMBPRINT
    } else {
        $environmentFile = Join-Path $repositoryRoot '.env.local'
        if (Test-Path -LiteralPath $environmentFile) {
            $thumbprintLines = @(Get-Content -LiteralPath $environmentFile | Where-Object {
                $_ -match '^\s*WNW_MSIX_CERTIFICATE_THUMBPRINT\s*='
            })
            if ($thumbprintLines.Count -gt 1) {
                throw "$environmentFile contains WNW_MSIX_CERTIFICATE_THUMBPRINT more than once."
            }
            if ($thumbprintLines.Count -eq 1) {
                $CertificateThumbprint = ($thumbprintLines[0] -replace '^\s*WNW_MSIX_CERTIFICATE_THUMBPRINT\s*=\s*', '').Trim().Trim('"', "'")
            }
        }
    }
}
if ([string]::IsNullOrWhiteSpace($CertificateThumbprint)) {
    throw 'Provide -CertificateThumbprint or set WNW_MSIX_CERTIFICATE_THUMBPRINT in .env.local.'
}

if (-not $PackagePath) {
    $package = Get-ChildItem -Path $packageDirectory -Filter 'What-N-When_*_x64.msix' -File -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if (-not $package) {
        throw "No MSIX package found in $packageDirectory. Run package-msix.ps1 first."
    }
    $PackagePath = $package.FullName
} else {
    $PackagePath = (Resolve-Path -LiteralPath $PackagePath).Path
}

$thumbprint = ($CertificateThumbprint -replace '\s', '').ToUpperInvariant()
if ($thumbprint -notmatch '^[0-9A-F]{40}$') {
    throw 'The certificate thumbprint must contain exactly 40 hexadecimal characters.'
}
$certificate = Get-ChildItem Cert:\CurrentUser\My |
    Where-Object { ($_.Thumbprint -replace '\s', '').ToUpperInvariant() -eq $thumbprint } |
    Select-Object -First 1
if (-not $certificate) {
    throw "Certificate $thumbprint was not found in Cert:\CurrentUser\My."
}
if (-not $certificate.HasPrivateKey) {
    throw "Certificate $thumbprint does not have an accessible private key."
}

Add-Type -AssemblyName System.IO.Compression.FileSystem
$archive = [System.IO.Compression.ZipFile]::OpenRead($PackagePath)
try {
    $manifestEntry = $archive.GetEntry('AppxManifest.xml')
    if (-not $manifestEntry) {
        throw "The package does not contain AppxManifest.xml: $PackagePath"
    }

    $reader = [System.IO.StreamReader]::new($manifestEntry.Open())
    try {
        [xml] $manifest = $reader.ReadToEnd()
    } finally {
        $reader.Dispose()
    }

    $identity = $manifest.SelectSingleNode("//*[local-name()='Identity']")
    if (-not $identity -or -not $identity.Publisher) {
        throw "The package manifest does not declare an identity publisher: $PackagePath"
    }
    if (-not [string]::Equals($certificate.Subject, $identity.Publisher, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Certificate subject '$($certificate.Subject)' does not match package publisher '$($identity.Publisher)'."
    }
} finally {
    $archive.Dispose()
}

$signTool = Get-Command signtool.exe -ErrorAction SilentlyContinue
if (-not $signTool) {
    $sdkRoots = @(
        $env:WindowsSdkDir,
        ${env:ProgramFiles(x86)},
        $env:ProgramFiles
    ) | Where-Object { $_ } | ForEach-Object {
        if ($_ -eq $env:WindowsSdkDir) {
            Join-Path $_ 'bin'
        } else {
            Join-Path $_ 'Windows Kits\10\bin'
        }
    }
    $signTool = Get-ChildItem -Path $sdkRoots -Filter 'signtool.exe' -File -Recurse -ErrorAction SilentlyContinue |
        Where-Object { $_.Directory.Name -eq 'x64' } |
        Sort-Object FullName -Descending |
        Select-Object -First 1
}
if (-not $signTool) {
    throw 'signtool.exe was not found in PATH or the Windows SDK.'
}
$signToolPath = if ($signTool.PSObject.Properties.Name -contains 'Source') { $signTool.Source } else { $signTool.FullName }

Write-Host "Signing $PackagePath"
& $signToolPath sign /fd SHA256 /sha1 $thumbprint /s My /tr $TimestampServer /td SHA256 $PackagePath
if ($LASTEXITCODE -ne 0) {
    throw "signtool.exe failed to sign the package (exit code $LASTEXITCODE)."
}

& $signToolPath verify /pa /v $PackagePath
if ($LASTEXITCODE -ne 0) {
    throw "signtool.exe could not verify the signed package (exit code $LASTEXITCODE)."
}

Write-Host "Signed and verified: $PackagePath"
