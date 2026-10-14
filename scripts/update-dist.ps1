# Invoked after linking, or manually with -Executable target/debug/rustgta3.exe.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repoRoot = Split-Path -Parent $PSScriptRoot

function Get-LinkValue([string[]]$LinkArguments, [string]$Name) {
    foreach ($argument in $LinkArguments) {
        if ($argument.StartsWith('@')) {
            # rustc uses a UTF-16 response file for long MSVC linker commands.
            $response = Get-Content -LiteralPath $argument.Substring(1).Trim('"') -Raw
            foreach ($line in $response -split '\r?\n') {
                $option = $line.Trim().Trim('"').Replace('\"', '"')
                if ($option.StartsWith($Name, [StringComparison]::OrdinalIgnoreCase)) {
                    return $option.Substring($Name.Length).Trim('"')
                }
            }
        } elseif ($argument.StartsWith($Name, [StringComparison]::OrdinalIgnoreCase)) {
            return $argument.Substring($Name.Length).Trim('"')
        }
    }
}

# PE import names identify the exact Bevy/Rust DLL hashes, including transitive
# and delay-loaded dependencies. No dumpbin or Visual Studio install is needed.
function Get-PeImports([string]$Path) {
    $reader = [IO.BinaryReader]::new([IO.File]::OpenRead($Path))
    try {
        if ($reader.ReadUInt16() -ne 0x5A4D) { throw "Invalid PE file: $Path" }
        $reader.BaseStream.Position = 0x3C
        $pe = $reader.ReadUInt32()
        $reader.BaseStream.Position = $pe
        if ($reader.ReadUInt32() -ne 0x4550) { throw "Invalid PE signature: $Path" }
        $reader.BaseStream.Position = $pe + 6
        $sectionCount = $reader.ReadUInt16()
        $reader.BaseStream.Position = $pe + 20
        $optionalSize = $reader.ReadUInt16()
        $optional = $pe + 24
        $reader.BaseStream.Position = $optional
        $magic = $reader.ReadUInt16()
        if ($magic -eq 0x20B) {
            $directories = $optional + 112
            $reader.BaseStream.Position = $optional + 24
            $imageBase = $reader.ReadUInt64()
        } elseif ($magic -eq 0x10B) {
            $directories = $optional + 96
            $reader.BaseStream.Position = $optional + 28
            $imageBase = $reader.ReadUInt32()
        } else { throw "Unsupported PE optional header: $Path" }
        $reader.BaseStream.Position = $directories - 4
        $directoryCount = $reader.ReadUInt32()
        $sections = @(
            for ($i = 0; $i -lt $sectionCount; $i++) {
                $reader.BaseStream.Position = $optional + $optionalSize + 40 * $i + 8
                $virtualSize = $reader.ReadUInt32()
                $rva = $reader.ReadUInt32()
                $rawSize = $reader.ReadUInt32()
                $offset = $reader.ReadUInt32()
                [pscustomobject]@{ Rva = $rva; Size = [Math]::Max($virtualSize, $rawSize); Offset = $offset }
            }
        )
        function Convert-Rva([long]$Rva) {
            foreach ($section in $sections) {
                if ($Rva -ge $section.Rva -and $Rva -lt $section.Rva + $section.Size) {
                    return [long]($section.Offset + $Rva - $section.Rva)
                }
            }
            throw "Invalid PE import RVA in $Path"
        }
        foreach ($table in @(
            @{ Index = 1; Stride = 20; NameOffset = 12 },
            @{ Index = 13; Stride = 32; NameOffset = 4 }
        )) {
            if ($table.Index -ge $directoryCount) { continue }
            $reader.BaseStream.Position = $directories + 8 * $table.Index
            $tableRva = $reader.ReadUInt32()
            $tableSize = $reader.ReadUInt32()
            if ($tableRva -eq 0) { continue }
            $tableOffset = Convert-Rva $tableRva
            for ($position = 0; $position + $table.Stride -le $tableSize; $position += $table.Stride) {
                $reader.BaseStream.Position = $tableOffset + $position
                $attributes = $reader.ReadUInt32()
                $reader.BaseStream.Position = $tableOffset + $position + $table.NameOffset
                [long]$nameRva = $reader.ReadUInt32()
                if ($nameRva -eq 0) { break }
                if ($table.Index -eq 13 -and ($attributes -band 1) -eq 0) { $nameRva -= $imageBase }
                $reader.BaseStream.Position = Convert-Rva $nameRva
                $name = [Text.StringBuilder]::new()
                while (($character = $reader.ReadByte()) -ne 0) { [void]$name.Append([char]$character) }
                $name.ToString()
            }
        }
    } finally { $reader.Dispose() }
}

function Copy-ChangedFile([string]$Source, [string]$Destination) {
    $sourceInfo = Get-Item -LiteralPath $Source
    if (Test-Path -LiteralPath $Destination -PathType Leaf) {
        $destinationInfo = Get-Item -LiteralPath $Destination
        if ($sourceInfo.Length -eq $destinationInfo.Length -and
            $sourceInfo.LastWriteTimeUtc -eq $destinationInfo.LastWriteTimeUtc) { return }
    }
    Copy-Item -LiteralPath $Source -Destination $Destination -Force
}

function Write-ChangedConfig([string]$Path, [string]$Content) {
    $contentWithNewline = $Content + [Environment]::NewLine
    if ((Test-Path -LiteralPath $Path) -and [IO.File]::ReadAllText($Path) -eq $contentWithNewline) { return }
    [IO.File]::WriteAllText($Path, $contentWithNewline, [Text.UTF8Encoding]::new($false))
}

function Get-ConfiguredPath([string]$Name) {
    $config = Join-Path $repoRoot "config/$Name"
    if (-not (Test-Path -LiteralPath $config -PathType Leaf)) { return }
    $line = [IO.File]::ReadAllLines($config) | Where-Object {
        $value = $_.Trim()
        $value.Length -gt 0 -and -not $value.StartsWith('#')
    } | Select-Object -First 1
    if (-not $line) { throw "Empty runtime configuration: $config" }
    $path = $line.Trim()
    if (-not [IO.Path]::IsPathRooted($path)) { $path = Join-Path (Split-Path $config) $path }
    [IO.Path]::GetFullPath($path)
}

try {
    $linkArguments = $args
    if ($args.Count -eq 2 -and $args[0] -eq '-Executable') {
        $Executable = $args[1]
        $linkArguments = @()
    } else { $Executable = Get-LinkValue $linkArguments '/OUT:' }
    if (-not $Executable) { throw 'Linker did not specify /OUT: for the game executable.' }
    $Executable = [IO.Path]::GetFullPath($Executable)
    $artifactName = [IO.Path]::GetFileNameWithoutExtension($Executable)
    # Cargo links hashed executables in deps; test harnesses must not replace games.
    if ($artifactName -notmatch '^(rustgta3|rustgta3newgame)(?:-[a-f0-9]+)?$') { exit 0 }
    $binaryName = $Matches[1]
    $compilerPath = 'rustc'
    if ($linkArguments.Count -gt 0) {
        # Binary unit tests have the same CARGO_BIN_NAME as the game. Inspect
        # this linker's compiler parent so a test executable never ships.
        $ancestorId = $PID
        for ($level = 0; $level -lt 5; $level++) {
            $process = Get-CimInstance Win32_Process -Filter "ProcessId=$ancestorId"
            if (-not $process) { throw 'Cannot identify the linker compiler process.' }
            if ($process.Name -eq 'rustc.exe') {
                if ($process.CommandLine -match '(?:^|\s)--test(?:\s|$)') { exit 0 }
                if ($process.ExecutablePath) { $compilerPath = $process.ExecutablePath }
                break
            }
            $ancestorId = $process.ParentProcessId
        }
    }
    $artifactDir = Split-Path $Executable
    $profileDir = $artifactDir
    if ((Split-Path $artifactDir -Leaf) -eq 'deps') { $profileDir = Split-Path $artifactDir }
    $sysroot = & $compilerPath --print sysroot
    if ($LASTEXITCODE -ne 0) { throw 'Cannot locate Rust runtime DLLs.' }
    $searchDirs = @($artifactDir, (Join-Path $profileDir 'deps'), $profileDir,
        (Join-Path $sysroot 'lib/rustlib/x86_64-pc-windows-msvc/lib'))
    $buildDir = Join-Path $profileDir 'build'
    if (Test-Path -LiteralPath $buildDir) {
        $searchDirs += @(Get-ChildItem -LiteralPath $buildDir -Directory | ForEach-Object { Join-Path $_.FullName 'out' })
    }
    $searchDirs += @($env:PATH.Split(';') | Where-Object { $_ })
    $files = @{}
    $files["$binaryName.exe"] = $Executable
    $gtaRoot = Get-ConfiguredPath 'original_gta3_path.txt'
    $files['LICENSE.pl_mpeg'] = Join-Path $repoRoot 'crates/rustgta3_formats/native/LICENSE.pl_mpeg'
    $files['LICENSE'] = Join-Path $repoRoot 'LICENSE'
    $files['THIRD_PARTY_NOTICES.md'] = Join-Path $repoRoot 'THIRD_PARTY_NOTICES.md'
    $queue = [Collections.Generic.Queue[string]]::new()
    $queue.Enqueue($Executable)
    while ($queue.Count -gt 0) {
        $owner = $queue.Dequeue()
        foreach ($dependency in Get-PeImports $owner) {
            if ($files.ContainsKey($dependency)) { continue }
            $dependencyLower = $dependency.ToLowerInvariant()
            if ($dependencyLower -match '^(api-ms-|ext-ms-)') { continue }
            $systemFile = Join-Path ([Environment]::SystemDirectory) $dependency
            if ((Test-Path -LiteralPath $systemFile) -and $dependencyLower -notmatch '^(vcruntime|msvcp|msvcr\d|concrt|vcomp)') { continue }
            $resolved = $null
            foreach ($directory in @((Split-Path $owner)) + $searchDirs + @([Environment]::SystemDirectory)) {
                $candidate = Join-Path $directory $dependency
                if (Test-Path -LiteralPath $candidate -PathType Leaf) { $resolved = $candidate; break }
            }
            if (-not $resolved) { throw "Missing runtime DLL '$dependency', imported by $owner" }
            $files[$dependency] = $resolved
            $queue.Enqueue($resolved)
        }
    }
    $destination = Join-Path $repoRoot "dist/$binaryName"
    [void][IO.Directory]::CreateDirectory((Join-Path $destination 'config'))
    foreach ($name in $files.Keys) {
        if ($name -ne "$binaryName.exe") { Copy-ChangedFile $files[$name] (Join-Path $destination $name) }
    }
    if ($gtaRoot) { Write-ChangedConfig (Join-Path $destination 'config/original_gta3_path.txt') $gtaRoot }
    Copy-ChangedFile $Executable (Join-Path $destination "$binaryName.exe")
    # Retire obsolete hashed DLLs only after the new executable is published.
    Get-ChildItem -LiteralPath $destination -Filter '*.dll' -File | Where-Object { -not $files.ContainsKey($_.Name) } |
        ForEach-Object { Remove-Item -LiteralPath $_.FullName }
    foreach ($obsolete in @('ffmpeg.exe', 'ffmpeg-LICENSE', 'ffmpeg-LICENSE.txt',
        'ffmpeg-COPYING', 'ffmpeg-COPYING.txt', 'ffmpeg-README.txt', 'config/ffmpeg_path.txt')) {
        $obsoletePath = Join-Path $destination $obsolete
        if (Test-Path -LiteralPath $obsoletePath -PathType Leaf) { Remove-Item -LiteralPath $obsoletePath }
    }
    $binaryMarker = Join-Path $destination '.cargo-dist'
    $createdMarker = -not (Test-Path -LiteralPath $binaryMarker)
    Write-ChangedConfig $binaryMarker ''
    $rootMarker = Join-Path $repoRoot 'dist/.cargo-dist'
    if (-not (Test-Path -LiteralPath $rootMarker)) {
        [IO.File]::Open($rootMarker, [IO.FileMode]::OpenOrCreate, [IO.FileAccess]::Write, [IO.FileShare]::ReadWrite).Dispose()
    }
    # Refresh Cargo's watched-folder list after a distribution is recreated.
    if ($createdMarker) { [IO.File]::SetLastWriteTimeUtc($rootMarker, [DateTime]::UtcNow) }
    if ($linkArguments.Count -eq 0) { [Console]::Error.WriteLine("rustgta3 dist: $destination") }
} catch {
    [Console]::Error.WriteLine("rustgta3 dist failed: $_")
    exit 1
}
