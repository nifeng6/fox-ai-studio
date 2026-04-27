$vsPath = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools"
$msvcVer = (Get-ChildItem "$vsPath\VC\Tools\MSVC" -Directory | Sort-Object Name -Descending | Select-Object -First 1).Name
$wkPath = "C:\Program Files (x86)\Windows Kits\10"
$wkVer = (Get-ChildItem "$wkPath\Lib" -Directory | Sort-Object Name -Descending | Select-Object -First 1).Name

$env:PATH = "$vsPath\VC\Tools\MSVC\$msvcVer\bin\Hostx64\x64;$wkPath\bin\$wkVer\x64;$env:PATH"
$env:LIB = "$vsPath\VC\Tools\MSVC\$msvcVer\lib\x64;$wkPath\Lib\$wkVer\um\x64;$wkPath\Lib\$wkVer\ucrt\x64"
$env:INCLUDE = "$vsPath\VC\Tools\MSVC\$msvcVer\include;$wkPath\Include\$wkVer\ucrt;$wkPath\Include\$wkVer\um;$wkPath\Include\$wkVer\shared"

Write-Host "MSVC $msvcVer + WinSDK $wkVer configured" -ForegroundColor Green
npx tauri dev
