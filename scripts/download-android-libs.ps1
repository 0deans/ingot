<#
.SYNOPSIS
    Downloads the Android native binaries (libpumpkin.so, libplayit.so, PRoot) for the Ingot Android build.

.DESCRIPTION
    Android 10+ enforces W^X - binaries downloaded at runtime to app data cannot be executed.
    The only solution is to bundle them in the APK as jniLibs. Because these files are large
    (Pumpkin ~116 MB, Playit ~10 MB) they are NOT committed to git. Run this script before
    building the Android APK.

    Output directory: src-tauri/gen/android/app/src/main/jniLibs/arm64-v8a/

.EXAMPLE
    .\scripts\download-android-libs.ps1
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$JniDir = Join-Path $PSScriptRoot "..\src-tauri\gen\android\app\src\main\jniLibs\arm64-v8a"
New-Item -ItemType Directory -Force -Path $JniDir | Out-Null

# ---- Pumpkin ----------------------------------------------------------------
$PumpkinDest = Join-Path $JniDir "libpumpkin.so"

if (Test-Path $PumpkinDest) {
    $szBytes = (Get-Item $PumpkinDest).Length
    $szMB = [math]::Round($szBytes / 1MB, 1)
    if ($szBytes -gt 10MB) {
        Write-Host "[libpumpkin.so] Already present ($szMB MB) - skipping." -ForegroundColor Green
    } else {
        Write-Warning "[libpumpkin.so] Existing file is too small ($szBytes bytes), re-downloading."
        Remove-Item $PumpkinDest -Force
    }
}

if (-not (Test-Path $PumpkinDest)) {
    Write-Host "[libpumpkin.so] Fetching latest Pumpkin release info..." -ForegroundColor Cyan
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/Pumpkin-MC/Pumpkin/releases/latest" `
        -Headers @{ "User-Agent" = "Ingot-Build-Script" }

    $asset = $release.assets | Where-Object {
        $_.name -match "aarch64-android" -and
        $_.name -notmatch "\.(sha256|md5)$" -and
        $_.name -notmatch "checksum"
    } | Select-Object -First 1

    if (-not $asset) {
        $assetNames = ($release.assets | Select-Object -ExpandProperty name) -join ", "
        throw "Could not find 'aarch64-android' asset in Pumpkin release $($release.tag_name). Assets: $assetNames"
    }

    $assetName = $asset.name
    $assetMB = [math]::Round($asset.size / 1MB, 1)
    Write-Host "[libpumpkin.so] Downloading $assetName ($assetMB MB)..." -ForegroundColor Cyan
    Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $PumpkinDest -UseBasicParsing
    $finalMB = [math]::Round((Get-Item $PumpkinDest).Length / 1MB, 1)
    Write-Host "[libpumpkin.so] Saved ($finalMB MB) -> $PumpkinDest" -ForegroundColor Green
}

# ---- Playit -----------------------------------------------------------------
$PlayitDest = Join-Path $JniDir "libplayit.so"

if (Test-Path $PlayitDest) {
    $szBytes = (Get-Item $PlayitDest).Length
    $szMB = [math]::Round($szBytes / 1MB, 1)
    if ($szBytes -gt 1MB) {
        Write-Host "[libplayit.so] Already present ($szMB MB) - skipping." -ForegroundColor Green
    } else {
        Write-Warning "[libplayit.so] Existing file is too small ($szBytes bytes), re-downloading."
        Remove-Item $PlayitDest -Force
    }
}

if (-not (Test-Path $PlayitDest)) {
    $PlayitUrl = "https://github.com/playit-cloud/playit-agent/releases/latest/download/playit-linux-aarch64"
    Write-Host "[libplayit.so] Downloading playit-linux-aarch64..." -ForegroundColor Cyan
    Invoke-WebRequest -Uri $PlayitUrl -OutFile $PlayitDest -UseBasicParsing
    $finalMB = [math]::Round((Get-Item $PlayitDest).Length / 1MB, 1)
    Write-Host "[libplayit.so] Saved ($finalMB MB) -> $PlayitDest" -ForegroundColor Green
}

# ---- PRoot (Termux build) ---------------------------------------------------
# Used to run Java servers inside an Alpine rootfs. Termux's proot is the build that
# works on modern Android: its loader must live in nativeLibraryDir (W^X), and it
# needs libtalloc + libandroid-shmem. Android only extracts lib*.so files, so each
# file is renamed; proot.rs symlinks libtalloc.so -> libtalloc.so.2 at runtime.
$ProotFiles = [ordered]@{
    "libproot.so"          = @{ Package = "proot";            Path = "bin/proot" }
    "libproot-loader.so"   = @{ Package = "proot";            Path = "libexec/proot/loader" }
    "libproot-loader32.so" = @{ Package = "proot";            Path = "libexec/proot/loader32" }
    "libtalloc.so"         = @{ Package = "libtalloc";        Path = "lib/libtalloc.so.2.*" }
    "libandroid-shmem.so"  = @{ Package = "libandroid-shmem"; Path = "lib/libandroid-shmem.so" }
}

$missingProot = @($ProotFiles.Keys | Where-Object { -not (Test-Path (Join-Path $JniDir $_)) })
if ($missingProot.Count -eq 0) {
    Write-Host "[proot] All PRoot libraries already present - skipping." -ForegroundColor Green
} else {
    $TermuxRepo = "https://packages.termux.dev/apt/termux-main"
    Write-Host "[proot] Fetching Termux package index..." -ForegroundColor Cyan
    $index = (Invoke-WebRequest -Uri "$TermuxRepo/dists/stable/main/binary-aarch64/Packages" -UseBasicParsing).Content
    if ($index -is [byte[]]) { $index = [System.Text.Encoding]::UTF8.GetString($index) }

    $WorkDir = Join-Path ([System.IO.Path]::GetTempPath()) "ingot-proot-$([guid]::NewGuid())"
    New-Item -ItemType Directory -Force -Path $WorkDir | Out-Null
    # Windows' bundled bsdtar understands both .deb (ar) and .tar.xz
    $Tar = Join-Path $env:SystemRoot "System32\tar.exe"

    try {
        foreach ($pkg in @($ProotFiles.Values | ForEach-Object { $_.Package } | Select-Object -Unique)) {
            $match = [regex]::Match($index, "(?ms)^Package: $([regex]::Escape($pkg))\r?\n.*?^Filename: (\S+)")
            if (-not $match.Success) { throw "Package '$pkg' not found in Termux index." }
            $pkgDir = Join-Path $WorkDir $pkg
            New-Item -ItemType Directory -Force -Path $pkgDir | Out-Null
            $deb = Join-Path $pkgDir "pkg.deb"
            Write-Host "[proot] Downloading $($match.Groups[1].Value)..." -ForegroundColor Cyan
            Invoke-WebRequest -Uri "$TermuxRepo/$($match.Groups[1].Value)" -OutFile $deb -UseBasicParsing
            Push-Location $pkgDir
            try {
                & $Tar -xf pkg.deb
                if ($LASTEXITCODE -ne 0) { throw "Failed to unpack $pkg.deb" }
                $dataTar = Get-ChildItem -Filter "data.tar.*" | Select-Object -First 1
                # Exit code ignored: symlinks may fail without Developer Mode; the
                # files we need are regular files and are verified below.
                & $Tar -xf $dataTar.Name 2>$null
            } finally {
                Pop-Location
            }
        }

        foreach ($name in $ProotFiles.Keys) {
            $spec = $ProotFiles[$name]
            $prefix = Join-Path $WorkDir "$($spec.Package)\data\data\com.termux\files\usr"
            $src = Get-ChildItem -Path (Join-Path $prefix $spec.Path) -File | Select-Object -First 1
            if (-not $src) { throw "File '$($spec.Path)' not found in package $($spec.Package)." }
            Copy-Item $src.FullName (Join-Path $JniDir $name) -Force
            Write-Host "[proot] $name <- $($spec.Package)/$($spec.Path)" -ForegroundColor Green
        }
    } finally {
        Remove-Item -Recurse -Force $WorkDir -ErrorAction SilentlyContinue
    }
}

Write-Host ""
$pMB = [math]::Round((Get-Item $PumpkinDest).Length / 1MB, 1)
$qMB = [math]::Round((Get-Item $PlayitDest).Length / 1MB, 1)
Write-Host "All Android native libraries are ready." -ForegroundColor Green
Write-Host "  libpumpkin.so : $pMB MB"
Write-Host "  libplayit.so  : $qMB MB"
