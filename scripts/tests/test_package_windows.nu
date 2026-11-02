#!/usr/bin/env nu
# Tests for scripts/ci/package_windows.nu
use std/assert
use ./runner.nu *
use ../ci/package_windows.nu [nsis-args]

def "test nsis-args without gui" [] {
    let a = (nsis-args "App" "app" "v1.2.3" "C:\\r" "target/release" "app-tui.exe" "")
    assert ("-DVERSION=1.2.3" in $a)
    assert ("-DBINDIR=target\\release" in $a)
    assert ("-DCLI_EXE=app-tui.exe" in $a)
    assert (not ($a | any {|x| $x | str starts-with "-DGUI_EXE"}))
}

def "test nsis-args with gui" [] {
    let a = (nsis-args "App" "app" "1.0.0" "C:\\r" "t" "c.exe" "g.exe")
    assert ("-DGUI_EXE=g.exe" in $a)
    assert ("-DVERSION=1.0.0" in $a)
}

def main [] { run-tests }
