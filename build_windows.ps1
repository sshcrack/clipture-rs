#!/usr/bin/env pwsh

$ErrorActionPreference = "Stop"
Push-Location

function wrapper() {

    # Build OBS Studio

    # Remove-Item -Force -Recurse build -ErrorAction SilentlyContinue
    cmake -S . -B build --preset windows-x64 `
    -DCMAKE_BUILD_TYPE=RelWithDebInfo `
    -DENABLE_BROWSER:BOOL=OFF `
    -DENABLE_VLC:BOOL=OFF `
    -DENABLE_UI:BOOL=OFF `
    -DENABLE_VST:BOOL=OFF `
    -DENABLE_SCRIPTING:BOOL=OFF `
    -DCOPIED_DEPENDENCIES:BOOL=OFF `
    -DCOPY_DEPENDENCIES:BOOL=ON `
    -DBUILD_FOR_DISTRIBUTION:BOOL=ON `
    -DENABLE_WEBSOCKET:BOOL=OFF `
    -DCMAKE_COMPILE_WARNING_AS_ERROR=OFF

    cmake --build build --config RelWithDebInfo

}

function Copy-Files() {
    Copy-Item -r ./build/rundir/RelWithDebInfo/* ../target/debug/ -Force
    Move-Item ../target/debug/bin/64bit/* ../target/debug/ -Force
}

try {
    Set-Location .\obs-studio
    wrapper
    Copy-Files
} catch {
    $_
    Write-Host $_.Exception.Message
    exit 1
} finally {
    Pop-Location
    exit 0
}