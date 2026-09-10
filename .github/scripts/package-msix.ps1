param(
    [string] $Version
)

$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$releaseDirectory = Join-Path $repositoryRoot 'src-tauri\target\release'
$stagingDirectory = Join-Path $releaseDirectory 'msix-staging'
$outputDirectory = Join-Path $releaseDirectory 'bundle\msix'
$configPath = Join-Path $repositoryRoot 'src-tauri\tauri.conf.json'
$iconPath = Join-Path $repositoryRoot 'web\assets\notion-calendar.ico'

if (-not $Version) {
    $Version = (Get-Content $configPath -Raw | ConvertFrom-Json).version
}

$versionParts = $Version.Split('-')[0].Split('.')
if ($versionParts.Count -gt 4 -or ($versionParts | Where-Object { $_ -notmatch '^\d+$' })) {
  throw "Version must contain one to four numeric components: $Version"
}
while ($versionParts.Count -lt 4) {
  $versionParts += '0'
}
$packageVersion = ($versionParts -join '.')

$executablePath = Join-Path $releaseDirectory 'notion-calendar-widget.exe'
if (-not (Test-Path $executablePath)) {
    throw "Release executable not found: $executablePath"
}

Remove-Item $stagingDirectory -Recurse -Force -ErrorAction SilentlyContinue
New-Item $stagingDirectory -ItemType Directory -Force | Out-Null
New-Item (Join-Path $stagingDirectory 'Assets') -ItemType Directory -Force | Out-Null
New-Item $outputDirectory -ItemType Directory -Force | Out-Null

Copy-Item $executablePath (Join-Path $stagingDirectory 'notion-calendar-widget.exe')

Add-Type -AssemblyName System.Drawing
$icon = [System.Drawing.Icon]::ExtractAssociatedIcon($iconPath)
$bitmap = $icon.ToBitmap()
try {
    $logoPath = Join-Path $stagingDirectory 'Assets\Logo.png'
    $bitmap.Save($logoPath, [System.Drawing.Imaging.ImageFormat]::Png)
} finally {
    $bitmap.Dispose()
    $icon.Dispose()
}

$manifest = @"
<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10" xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10" xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedCapabilities">
  <Identity Name="ca.willryan.notioncalendarwidget" Publisher="CN=Notion Calendar Widget" Version="$packageVersion" />
  <Properties>
    <DisplayName>Notion Calendar Widget</DisplayName>
    <PublisherDisplayName>Will Ryan</PublisherDisplayName>
    <Logo>Assets\Logo.png</Logo>
  </Properties>
  <Resources>
    <Resource Language="en-us" />
  </Resources>
  <Applications>
    <Application Id="NotionCalendarWidget" Executable="notion-calendar-widget.exe" EntryPoint="Windows.FullTrustApplication">
      <uap:VisualElements AppListEntry="none" DisplayName="Notion Calendar Widget" Description="Notion Calendar desktop widget" BackgroundColor="#1a1a1a" Square150x150Logo="Assets\Logo.png" Square44x44Logo="Assets\Logo.png" />
    </Application>
  </Applications>
  <Capabilities>
    <rescap:Capability Name="runFullTrust" />
  </Capabilities>
</Package>
"@

Set-Content (Join-Path $stagingDirectory 'AppxManifest.xml') $manifest -Encoding utf8

$makeAppx = Get-Command makeappx.exe -ErrorAction SilentlyContinue
if (-not $makeAppx) {
  $sdkRoots = @(
    $env:WindowsSdkDir,
    ${env:ProgramFiles(x86)},
    $env:ProgramFiles
  ) | Where-Object { $_ } | ForEach-Object {
    Join-Path $_ 'Windows Kits\10\bin'
  }
  $makeAppx = Get-ChildItem -Path $sdkRoots -Filter 'makeappx.exe' -File -Recurse -ErrorAction SilentlyContinue |
    Where-Object { $_.Directory.Name -eq 'x64' } |
    Sort-Object FullName -Descending |
    Select-Object -First 1
}
if (-not $makeAppx) {
    throw 'makeappx.exe was not found in PATH or the Windows SDK.'
}
$makeAppxPath = if ($makeAppx.PSObject.Properties.Name -contains 'Source') { $makeAppx.Source } else { $makeAppx.FullName }

$packagePath = Join-Path $outputDirectory "Notion.Calendar.Widget_${packageVersion}_x64.msix"
Remove-Item $packagePath -Force -ErrorAction SilentlyContinue
& $makeAppxPath pack /d $stagingDirectory /p $packagePath /nv
if ($LASTEXITCODE -ne 0) {
    throw "makeappx.exe failed with exit code $LASTEXITCODE"
}