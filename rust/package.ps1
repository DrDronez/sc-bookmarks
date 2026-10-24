param([string]$Destination = (Join-Path $PSScriptRoot 'dist/SC-Bookmarks-portable'))
$ErrorActionPreference='Stop'
$root=$PSScriptRoot
if(Test-Path (Join-Path $Destination 'navigator-data-rust')){throw 'Use a fresh package destination: this folder contains user data.'}
if (!(Test-Path (Join-Path $root 'target/release/sc-bookmarks.exe'))) { throw 'Build with cargo build --release first.' }
New-Item -ItemType Directory -Force $Destination,(Join-Path $Destination 'runtime'),(Join-Path $Destination 'licenses') | Out-Null
Copy-Item (Join-Path $root 'target/release/sc-bookmarks.exe') (Join-Path $Destination 'SC-Bookmarks.exe')
Copy-Item (Join-Path $root 'runtime/*') (Join-Path $Destination 'runtime')
# App-local VC runtime also resolves dependencies of the executable and loaded ORT DLL.
Copy-Item (Join-Path $root 'runtime/*140*.dll') $Destination
Copy-Item (Join-Path $root '../LICENSE'),(Join-Path $root '../NOTICE') $Destination
Copy-Item (Join-Path $root 'PORTABLE.md') (Join-Path $Destination 'README.md')
Copy-Item (Join-Path $root 'licenses/*') (Join-Path $Destination 'licenses')
$metadataText=& cargo metadata --manifest-path (Join-Path $root 'Cargo.toml') --format-version 1 --locked --offline --filter-platform x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0){throw 'Cargo metadata failed'}
$metadata=$metadataText | ConvertFrom-Json
$inventory=@('# Rust dependency notices','','Each component retains its own license. SC Bookmarks code is PolyForm Noncommercial; third-party licenses are not replaced.','')
foreach($package in $metadata.packages){
 if($package.name -eq 'sc-bookmarks'){continue}
 $inventory+="- $($package.name) $($package.version): $($package.license)"
 $base=Split-Path $package.manifest_path
 $licenseDir=Join-Path $Destination "licenses/$($package.name)-$($package.version)"
 $files=Get-ChildItem -LiteralPath $base -File -Recurse | Where-Object { $_.Name -match '(?i)^(license|licence|notice|copyright|ofl)([._-]|$)' }
 foreach($file in $files){$relative=[IO.Path]::GetRelativePath($base,$file.FullName);$target=Join-Path $licenseDir $relative;New-Item -ItemType Directory -Force (Split-Path $target) | Out-Null;Copy-Item -LiteralPath $file.FullName -Destination $target}
}
$inventory | Set-Content -Encoding utf8 (Join-Path $Destination 'licenses/RUST-DEPENDENCIES.md')
Copy-Item (Join-Path $root 'THIRD_PARTY_NOTICES.md') $Destination
$hashes=Get-ChildItem -LiteralPath $Destination -File -Recurse | Where-Object Name -ne 'SHA256SUMS.txt' | ForEach-Object {"$((Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLower())  $([IO.Path]::GetRelativePath($Destination,$_.FullName))"}
$hashes | Set-Content -Encoding ascii (Join-Path $Destination 'SHA256SUMS.txt')
Write-Output "Portable package: $Destination"
