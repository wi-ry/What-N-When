param(
    [string] $Version
)

$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$releaseDirectory = Join-Path $repositoryRoot 'src-tauri\target\release'
$stagingDirectory = Join-Path $releaseDirectory 'msix-staging'
$providerProject = Join-Path $repositoryRoot 'src-tauri\widget-provider\WhatNWhenWidgetProvider.csproj'
$providerPublishDirectory = Join-Path $releaseDirectory 'widget-provider-publish'
$outputDirectory = Join-Path $releaseDirectory 'bundle\msix'
$configPath = Join-Path $repositoryRoot 'src-tauri\tauri.conf.json'
$iconPath = Join-Path $repositoryRoot 'web\assets\icon.ico'

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

$executablePath = Join-Path $releaseDirectory 'what-n-when.exe'
if (-not (Test-Path $executablePath)) {
    throw "Release executable not found: $executablePath"
}

if (-not (Test-Path $providerProject)) {
    throw "Widget provider project not found: $providerProject"
}
if (Test-Path $providerPublishDirectory) {
    Remove-Item $providerPublishDirectory -Recurse -Force
}
dotnet publish $providerProject --configuration Release --runtime win-x64 --self-contained true --output $providerPublishDirectory -p:WindowsAppSDKSelfContained=true
if ($LASTEXITCODE -ne 0) {
    throw "Widget provider publish failed with exit code $LASTEXITCODE"
}

Remove-Item $stagingDirectory -Recurse -Force -ErrorAction SilentlyContinue
New-Item $stagingDirectory -ItemType Directory -Force | Out-Null
New-Item (Join-Path $stagingDirectory 'Assets') -ItemType Directory -Force | Out-Null
New-Item $outputDirectory -ItemType Directory -Force | Out-Null

Copy-Item $executablePath (Join-Path $stagingDirectory 'what-n-when.exe')
Copy-Item (Join-Path $providerPublishDirectory '*') $stagingDirectory -Recurse -Force

Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @'
using System.Runtime.InteropServices;

public static class WidgetPreviewAccent {
    [DllImport("dwmapi.dll", ExactSpelling = true)]
    public static extern int DwmGetColorizationColor(
        out uint colorizationColor,
        [MarshalAs(UnmanagedType.Bool)] out bool opaqueBlend);
}
'@
$icon = [System.Drawing.Icon]::new($iconPath, 256, 256)
$bitmap = [System.Drawing.Bitmap]::new(256, 256)
$iconGraphics = [System.Drawing.Graphics]::FromImage($bitmap)
try {
    $iconGraphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $iconGraphics.DrawIcon($icon, [System.Drawing.Rectangle]::new(0, 0, 256, 256))
    $logoPath = Join-Path $stagingDirectory 'Assets\Logo.png'
    $bitmap.Save($logoPath, [System.Drawing.Imaging.ImageFormat]::Png)
    $providerIconPath = Join-Path $stagingDirectory 'Assets\ProviderIcon.png'
    $bitmap.Save($providerIconPath, [System.Drawing.Imaging.ImageFormat]::Png)
} finally {
    $iconGraphics.Dispose()
    $bitmap.Dispose()
    $icon.Dispose()
}

$preview = [System.Drawing.Bitmap]::new(600, 608)
$graphics = [System.Drawing.Graphics]::FromImage($preview)
$background = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(24, 24, 24))
$card = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(42, 42, 42))
$calendar = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(55, 55, 55))
$primaryText = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::White)
$secondaryText = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(190, 190, 190))
[uint32]$colorizationColor = 0
[bool]$opaqueBlend = $false
$dwmResult = [WidgetPreviewAccent]::DwmGetColorizationColor([ref]$colorizationColor, [ref]$opaqueBlend)
if ($dwmResult -ne 0) {
    throw "Could not read the Windows accent color (HRESULT $dwmResult)."
}
$accentColor = [System.Drawing.Color]::FromArgb(
    [int](($colorizationColor -shr 24) -band 0xFF),
    [int](($colorizationColor -shr 16) -band 0xFF),
    [int](($colorizationColor -shr 8) -band 0xFF),
    [int]($colorizationColor -band 0xFF))
$accent = [System.Drawing.SolidBrush]::new($accentColor)
$separatorPen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(100, 100, 100), 1)
$dateFont = [System.Drawing.Font]::new('Segoe UI', 32, [System.Drawing.FontStyle]::Regular)
$weekdayFont = [System.Drawing.Font]::new('Segoe UI', 12, [System.Drawing.FontStyle]::Regular)
$dayFont = [System.Drawing.Font]::new('Segoe UI', 16, [System.Drawing.FontStyle]::Regular)
$todayFont = [System.Drawing.Font]::new('Segoe UI', 16, [System.Drawing.FontStyle]::Bold)
$monthFont = [System.Drawing.Font]::new('Segoe UI', 14, [System.Drawing.FontStyle]::Regular)
$summaryWeekdayFont = [System.Drawing.Font]::new('Segoe UI', 20, [System.Drawing.FontStyle]::Bold)
$smallFont = [System.Drawing.Font]::new('Segoe UI', 12, [System.Drawing.FontStyle]::Regular)
$centered = [System.Drawing.StringFormat]::new()
$centered.Alignment = [System.Drawing.StringAlignment]::Center
$centered.LineAlignment = [System.Drawing.StringAlignment]::Center
try {
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $graphics.FillRectangle($background, 0, 0, 600, 608)
    $graphics.FillRectangle($card, 24, 18, 552, 572)
    $today = Get-Date
    $graphics.DrawString([string]$today.Day, $dateFont, $accent, 48, 22)
    $graphics.DrawString($today.ToString('MMMM'), $monthFont, $accent, 52, 76)
    $graphics.DrawLine($separatorPen, 205, 34, 205, 92)
    $graphics.DrawString($today.ToString('dddd'), $summaryWeekdayFont, $primaryText, 226, 46)
    $graphics.FillRectangle($calendar, 40, 112, 520, 440)

    $culture = [System.Globalization.CultureInfo]::CurrentCulture
    $firstDayOfWeek = [int]$culture.DateTimeFormat.FirstDayOfWeek
    $gridLeft = 48
    $cellWidth = 72
    for ($column = 0; $column -lt 7; $column++) {
        $dayOfWeek = [System.DayOfWeek](($firstDayOfWeek + $column) % 7)
        $weekday = $culture.DateTimeFormat.GetAbbreviatedDayName($dayOfWeek).Substring(0, 1)
        $x = $gridLeft + $column * $cellWidth
        $graphics.DrawString($weekday, $weekdayFont, $secondaryText, [System.Drawing.RectangleF]::new($x, 118, $cellWidth, 28), $centered)
    }

    $firstOfMonth = [datetime]::new($today.Year, $today.Month, 1)
    $offset = (([int]$firstOfMonth.DayOfWeek - $firstDayOfWeek) + 7) % 7
    for ($cell = 0; $cell -lt 42; $cell++) {
        $date = $firstOfMonth.AddDays($cell - $offset)
        $column = $cell % 7
        $row = [math]::Floor($cell / 7)
        $x = $gridLeft + $column * $cellWidth
        $y = 150 + $row * 64
        $isToday = $date.Date -eq $today.Date
        $font = if ($isToday) { $todayFont } else { $dayFont }
        $brush = if ($isToday) { $primaryText } elseif ($date.Month -ne $today.Month) { $secondaryText } else { $primaryText }
        if ($isToday) {
            $graphics.FillEllipse($accent, $x + 16, $y + 2, 40, 40)
        }
        $graphics.DrawString([string]$date.Day, $font, $brush, [System.Drawing.RectangleF]::new($x, $y + 1, $cellWidth, 42), $centered)
    }

    $graphics.DrawString('Open calendar', $smallFont, $accent, [System.Drawing.RectangleF]::new(24, 554, 552, 24), $centered)
    $preview.Save((Join-Path $stagingDirectory 'Assets\WidgetPreview.png'), [System.Drawing.Imaging.ImageFormat]::Png)
} finally {
    $preview.Dispose()
    $graphics.Dispose()
    $background.Dispose()
    $card.Dispose()
    $calendar.Dispose()
    $separatorPen.Dispose()
    $primaryText.Dispose()
    $secondaryText.Dispose()
    $accent.Dispose()
    $dateFont.Dispose()
    $weekdayFont.Dispose()
    $dayFont.Dispose()
    $todayFont.Dispose()
    $monthFont.Dispose()
    $summaryWeekdayFont.Dispose()
    $smallFont.Dispose()
    $centered.Dispose()
}

$manifestTemplatePath = Join-Path $PSScriptRoot 'AppxManifest.template.xml'
$manifest = (Get-Content $manifestTemplatePath -Raw) -replace '\{\{PackageVersion\}\}', $packageVersion

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

$packagePath = Join-Path $outputDirectory "What-N-When_${packageVersion}_x64.msix"
Remove-Item $packagePath -Force -ErrorAction SilentlyContinue
& $makeAppxPath pack /d $stagingDirectory /p $packagePath /nv
if ($LASTEXITCODE -ne 0) {
    throw "makeappx.exe failed with exit code $LASTEXITCODE"
}