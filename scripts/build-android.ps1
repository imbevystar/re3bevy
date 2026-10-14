[CmdletBinding()]
param(
    [string]$GtaDir,
    [string]$SdkDir,
    [switch]$BundleGameData,
    [switch]$Check,
    [switch]$SetupOnly,
    [switch]$PackageOnly
)

$ErrorActionPreference = 'Stop'
$repository = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$ndkVersion = '28.2.13676358'
$gradleVersion = '8.7'
$savedEnvironment = @{}
foreach ($name in @('ANDROID_HOME','ANDROID_SDK_ROOT','ANDROID_NDK_HOME','JAVA_HOME','CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS')) {
    $savedEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
}
$temporary = $null
Push-Location $repository
try {
    $metadataJson = & cargo metadata --no-deps --format-version 1
    if ($LASTEXITCODE) { throw 'Cannot locate the Cargo workspace output directory.' }
    $targetRoot = ($metadataJson | ConvertFrom-Json).target_directory
    if ($GtaDir -and !$BundleGameData) {
        throw '-GtaDir requires -BundleGameData for a private APK. The default APK imports user-provided data on the phone.'
    }
    $files = @()
    $total = [long]0
    if ($BundleGameData) {
        if (!$GtaDir) {
            $config = Join-Path $repository 'config/original_gta3_path.txt'
            if (!(Test-Path -LiteralPath $config)) { throw 'Set config/original_gta3_path.txt or pass -GtaDir <GTA III installation>.' }
            $GtaDir = Get-Content -LiteralPath $config | Where-Object { $_.Trim() -and !$_.Trim().StartsWith('#') } | Select-Object -First 1
            if (!$GtaDir) { throw 'The original GTA III path configuration is empty.' }
            $GtaDir = $GtaDir.Trim()
            if (![IO.Path]::IsPathRooted($GtaDir)) { $GtaDir = Join-Path (Split-Path $config) $GtaDir }
        }
        $GtaDir = (Resolve-Path -LiteralPath $GtaDir).Path
        $directories = @('anim','audio','data','models','movies','mp3','skins','text','txd')
        foreach ($required in @('models/coll/peds.col','models/gta3.img','models/gta3.dir','data/default.dat','data/gta3.dat','audio/sfx.raw','audio/sfx.sdt')) {
            if (!(Test-Path -LiteralPath (Join-Path $GtaDir $required) -PathType Leaf)) { throw "Missing original GTA III data: $required" }
        }
        $files = @(foreach ($directory in $directories) {
            $source = Join-Path $GtaDir $directory
            if (Test-Path -LiteralPath $source -PathType Container) { Get-ChildItem -LiteralPath $source -File -Recurse }
        })
        $total = [long](($files | Measure-Object -Property Length -Sum).Sum)
        if ($total -gt 1900MB) { throw 'Original data exceeds the single-APK size budget. Remove optional user MP3s/skins from the installation selected for this build.' }
        Write-Warning "PRIVATE APK: includes $($files.Count) original data files ($([math]::Round($total/1MB)) MB). Do not upload or redistribute."
    } else {
        Write-Host 'ARM64 APK without game data; the launcher imports the user-selected installation on the phone.'
    }

    if (!$SdkDir) {
        $SdkDir = $env:ANDROID_HOME
        if (!$SdkDir) { $SdkDir = $env:ANDROID_SDK_ROOT }
        if (!$SdkDir) { $SdkDir = Join-Path $env:LOCALAPPDATA 'Android/Sdk' }
    }
    $SdkDir = (Resolve-Path -LiteralPath $SdkDir).Path
    $env:ANDROID_HOME = $SdkDir
    $env:ANDROID_SDK_ROOT = $SdkDir
    $env:ANDROID_NDK_HOME = Join-Path $SdkDir "ndk/$ndkVersion"
    $java = Get-Command java -ErrorAction SilentlyContinue
    if (!$env:JAVA_HOME -and $java) { $env:JAVA_HOME = Split-Path (Split-Path $java.Source) }
    if (!$env:JAVA_HOME) {
        $studioJava = Join-Path ${env:ProgramFiles} 'Android/Android Studio/jbr'
        if (Test-Path -LiteralPath "$studioJava/bin/java.exe") { $env:JAVA_HOME = $studioJava }
    }
    if (!$env:JAVA_HOME -or !(Test-Path -LiteralPath "$env:JAVA_HOME/bin/java.exe")) { throw 'Install JDK 17 or 21, or set JAVA_HOME to Android Studio/jbr.' }
    $javaProperties = & "$env:JAVA_HOME/bin/java.exe" -XshowSettings:properties -version 2>&1 | Out-String
    if ($javaProperties -notmatch 'java\.specification\.version\s*=\s*(17|21)\b') { throw 'This Android build uses JDK 17 or 21.' }
    $gradleCache = Join-Path $env:USERPROFILE ".gradle/rust-gta-3/gradle-$gradleVersion"
    $gradle = Join-Path $gradleCache 'bin/gradle.bat'
    if ($Check) {
        foreach ($path in @('platforms/android-35/android.jar','build-tools/35.0.0/aapt2.exe',"ndk/$ndkVersion/source.properties")) {
            Write-Host "$path : $(Test-Path -LiteralPath (Join-Path $SdkDir $path))"
        }
        Write-Host "cargo-ndk : $([bool](Get-Command cargo-ndk -ErrorAction SilentlyContinue))"
        Write-Host "Gradle $gradleVersion : $(Test-Path -LiteralPath $gradle)"
        Write-Host 'Missing tools are installed by the normal command or -SetupOnly. No game compilation was started.'
        return
    }

    $sdkManager = Join-Path $SdkDir 'cmdline-tools/latest/bin/sdkmanager.bat'
    if (!(Test-Path -LiteralPath $sdkManager)) {
        $sdkManager = Get-ChildItem -LiteralPath (Join-Path $SdkDir 'cmdline-tools') -Filter sdkmanager.bat -Recurse | Select-Object -First 1 -ExpandProperty FullName
    }
    if (!$sdkManager) { throw 'Install Android SDK Command-line Tools using Android Studio.' }
    $missingPackages = @()
    if (!(Test-Path -LiteralPath (Join-Path $SdkDir 'platforms/android-35/android.jar'))) { $missingPackages += 'platforms;android-35' }
    if (!(Test-Path -LiteralPath (Join-Path $SdkDir 'build-tools/35.0.0/aapt2.exe'))) { $missingPackages += 'build-tools;35.0.0' }
    if (!(Test-Path -LiteralPath (Join-Path $env:ANDROID_NDK_HOME 'source.properties'))) { $missingPackages += "ndk;$ndkVersion" }
    if ($missingPackages.Count) {
        & $sdkManager "--sdk_root=$SdkDir" @missingPackages
        if ($LASTEXITCODE) { throw 'Android SDK setup failed. Accept the SDK licenses in Android Studio and rerun.' }
    }
    if (!$PackageOnly) {
        & rustup target add aarch64-linux-android
        if ($LASTEXITCODE) { throw 'Rust ARM64 target setup failed.' }
        if (!(Get-Command cargo-ndk -ErrorAction SilentlyContinue)) {
            & cargo install cargo-ndk --version 4.1.2 --locked
            if ($LASTEXITCODE) { throw 'cargo-ndk installation failed.' }
        }
    }

    if (!(Test-Path -LiteralPath $gradle)) {
        $temporary = Join-Path ([IO.Path]::GetTempPath()) ('rust-gta-3-android-' + [Guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Path $temporary | Out-Null
        $zip = Join-Path $temporary 'gradle.zip'
        $url = "https://services.gradle.org/distributions/gradle-$gradleVersion-bin.zip"
        Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing
        $expected = '544c35d6bd849ae8a5ed0bcea39ba677dc40f49df7d1835561582da2009b961d'
        if ((Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash -ne $expected) { throw 'Gradle download checksum mismatch.' }
        $cacheParent = Split-Path $gradleCache
        New-Item -ItemType Directory -Path $cacheParent -Force | Out-Null
        Expand-Archive -LiteralPath $zip -DestinationPath $cacheParent -Force
    }
    if ($SetupOnly) { Write-Host 'Android toolchain ready. No game compilation was started.'; return }
    if (!$temporary) {
        $temporary = Join-Path ([IO.Path]::GetTempPath()) ('rust-gta-3-android-' + [Guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Path $temporary | Out-Null
    }
    $nativeLibs = Join-Path $temporary 'jniLibs'
    $compiledLibrary = Join-Path $targetRoot 'aarch64-linux-android/release/librust_gta_3.so'
    if ($PackageOnly) {
        if (!(Test-Path -LiteralPath $compiledLibrary -PathType Leaf)) { throw 'No compiled ARM64 library found. Run without -PackageOnly first.' }
        Write-Host 'Packaging the existing ARM64 build; Rust compilation skipped.'
    } else {
        # Keep Cargo output in its default cache; align for 16 KB Android devices.
        $env:CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS = '-C link-arg=-Wl,-z,max-page-size=16384 -C link-arg=-Wl,-z,common-page-size=16384'
        # Request the shared library only for Android, preserving desktop builds.
        & cargo ndk -t arm64-v8a -P 26 rustc -p rust-gta-3 --lib --crate-type cdylib --release --locked
        if ($LASTEXITCODE) { throw 'Android Rust build failed.' }
    }
    $nativeLibrary = Join-Path $nativeLibs 'arm64-v8a/librust_gta_3.so'
    New-Item -ItemType Directory -Path (Split-Path $nativeLibrary) -Force | Out-Null
    Copy-Item -LiteralPath $compiledLibrary -Destination $nativeLibrary
    & (Join-Path $env:ANDROID_NDK_HOME 'toolchains/llvm/prebuilt/windows-x86_64/bin/llvm-strip.exe') --strip-unneeded $nativeLibrary
    if ($LASTEXITCODE) { throw 'Native library stripping failed.' }

    $assetRoot = Join-Path $temporary 'assets'
    New-Item -ItemType Directory -Path $assetRoot -Force | Out-Null
    if ($BundleGameData) {
        $originalRoot = Join-Path $assetRoot 'gta3'
        New-Item -ItemType Directory -Path $originalRoot -Force | Out-Null
        $revisionParts = foreach ($file in $files | Sort-Object FullName) {
            $relative = $file.FullName.Substring($GtaDir.TrimEnd('\','/').Length).TrimStart('\','/')
            $destination = Join-Path $originalRoot $relative
            New-Item -ItemType Directory -Path (Split-Path $destination) -Force | Out-Null
            Copy-Item -LiteralPath $file.FullName -Destination $destination
            "$relative $($file.Length) $((Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash)"
        }
        $sha = [Security.Cryptography.SHA256]::Create()
        try { $digest = [BitConverter]::ToString($sha.ComputeHash([Text.Encoding]::UTF8.GetBytes(($revisionParts -join "`n")))).Replace('-','').ToLowerInvariant() }
        finally { $sha.Dispose() }
        [IO.File]::WriteAllText((Join-Path $assetRoot 'gta3-revision'), "$digest $total", [Text.UTF8Encoding]::new($false))
    }
    Copy-Item -LiteralPath (Join-Path $repository 'LICENSE') -Destination (Join-Path $assetRoot 'LICENSE')
    Copy-Item -LiteralPath (Join-Path $repository 'THIRD_PARTY_NOTICES.md') -Destination (Join-Path $assetRoot 'THIRD_PARTY_NOTICES.md')
    Copy-Item -LiteralPath (Join-Path $repository 'crates/rust_gta_3_formats/native/LICENSE.pl_mpeg') -Destination (Join-Path $assetRoot 'LICENSE.pl_mpeg')
    $privateData = $BundleGameData.IsPresent.ToString().ToLowerInvariant()
    & $gradle -p (Join-Path $repository 'android') --project-cache-dir (Join-Path $targetRoot 'android/.gradle') --no-daemon "-PcargoTarget=$targetRoot" "-PgameAssets=$assetRoot" "-PnativeLibs=$nativeLibs" "-PprivateGameData=$privateData" :app:assembleDebug
    if ($LASTEXITCODE) { throw 'APK packaging failed.' }
    $apk = Join-Path $targetRoot 'android/app/outputs/apk/debug/app-debug.apk'
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $package = [IO.Compression.ZipFile]::OpenRead($apk)
    try {
        $assetNames = @($package.Entries | Where-Object { $_.FullName.StartsWith('assets/') -and $_.Name } | ForEach-Object { $_.FullName.Substring(7) })
        foreach ($notice in @('LICENSE','THIRD_PARTY_NOTICES.md','LICENSE.pl_mpeg')) {
            if ($assetNames -notcontains $notice) { throw "APK is missing notice: $notice" }
        }
        if (!$BundleGameData -and @($assetNames | Where-Object { $_ -notin @('LICENSE','THIRD_PARTY_NOTICES.md','LICENSE.pl_mpeg') }).Count) {
            throw 'Default APK unexpectedly contains assets other than license notices.'
        }
    } finally { $package.Dispose() }
    $distribution = Join-Path $repository 'dist/android'
    New-Item -ItemType Directory -Path $distribution -Force | Out-Null
    # Preserve legacy builds whose generic filename hid bundled game data.
    $defaultApk = Join-Path $distribution 'rust-gta-3-arm64.apk'
    if (!$BundleGameData -and (Test-Path -LiteralPath $defaultApk -PathType Leaf)) {
        $legacy = [IO.Compression.ZipFile]::OpenRead($defaultApk)
        try { $hasGameData = @($legacy.Entries | Where-Object { $_.FullName.StartsWith('assets/gta3/') }).Count -gt 0 }
        finally { $legacy.Dispose() }
        if ($hasGameData) {
            $privateApk = Join-Path $distribution 'rust-gta-3-arm64-private-with-game-data.apk'
            if (Test-Path -LiteralPath $privateApk) {
                $privateApk = Join-Path $distribution ('rust-gta-3-arm64-private-with-game-data-' + [Guid]::NewGuid().ToString('N') + '.apk')
            }
            Move-Item -LiteralPath $defaultApk -Destination $privateApk
            Write-Host "Preserved previous private APK: $privateApk"
        }
    }
    $apkName = if ($BundleGameData) { 'rust-gta-3-arm64-private-with-game-data.apk' } else { 'rust-gta-3-arm64.apk' }
    Move-Item -LiteralPath $apk -Destination (Join-Path $distribution $apkName) -Force
    Write-Host "APK ready: $distribution/$apkName"
} finally {
    Pop-Location
    foreach ($name in $savedEnvironment.Keys) { [Environment]::SetEnvironmentVariable($name, $savedEnvironment[$name], 'Process') }
    if ($temporary -and (Test-Path -LiteralPath $temporary)) {
        $resolved = [IO.Path]::GetFullPath($temporary)
        $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\'
        if (!$resolved.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase) -or !(Split-Path $resolved -Leaf).StartsWith('rust-gta-3-android-')) { throw 'Unsafe Android temporary cleanup path.' }
        Remove-Item -LiteralPath $resolved -Recurse -Force
    }
}
