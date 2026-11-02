#!/usr/bin/env nu
# Windows NSIS installer from a release build. Needs NSIS (`choco install nsis`).
# usage: nu scripts/ci/package_windows.nu <version> <bindir> [--gui <exe>]
#   e.g. nu scripts/ci/package_windows.nu v1.2.3 target/release
#   -> dist/flashkraft-v<v>-windows-x86_64-setup.exe

const APP = "FlashKraft"
const SLUG = "flashkraft"
const CLI = "flashkraft-tui.exe"

# makensis command-line defines (see packaging/windows/installer.nsi).
export def nsis-args [app: string, slug: string, version: string, root: string, bindir: string, cli: string, gui: string] {
    let base = [
        $"-DAPP_NAME=($app)"
        $"-DSLUG=($slug)"
        $"-DVERSION=($version | str replace --regex '^v' '')"
        $"-DROOT=($root)"
        $"-DBINDIR=($bindir | str replace --all '/' '\')"
        $"-DCLI_EXE=($cli)"
    ]
    if $gui == "" { $base } else { $base | append $"-DGUI_EXE=($gui)" }
}

def main [version: string, bindir: string, --gui: string = ""] {
    mkdir dist
    let makensis = (if (which makensis | is-empty) { 'C:\Program Files (x86)\NSIS\makensis.exe' } else { "makensis" })
    ^$makensis ...(nsis-args $APP $SLUG $version $env.PWD $bindir $CLI $gui) packaging/windows/installer.nsi
}
