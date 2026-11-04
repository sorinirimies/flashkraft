#!/usr/bin/env nu
# ── FlashKraft · test_examples.nu ────────────────────────────────────
# The demo environment and the screenshot helpers under examples/.

use std/assert
use runner.nu *
use ../../examples/vhs/setup.nu [settings-files]
use ../../examples/capture/gui.nu [rect]
use ../../examples/capture/web.nu [shot-args default-browser]

const ROOT = path self ../..

def "test demo: every settings file of both config dirs gets the theme" [] {
    let files = settings-files /h "Tokyo Night"
    assert equal ($files | length) 4
    assert equal ($files.path | where ($it | str starts-with "/h/.config/flashkraft/") | length) 2
    assert equal ($files.path | where ($it | str starts-with "/h/Library/Application Support/flashkraft/") | length) 2
    assert equal ($files.text | uniq) ["{\"theme\":\"Tokyo Night\"}\n"]
}

def "test demo: the example scripts parse" [] {
    for f in [examples/vhs/setup.nu examples/capture/gui.nu examples/capture/web.nu] {
        assert (nu-check ($ROOT | path join $f)) $f
    }
}

def "test demo: the tapes load the Nushell demo environment" [] {
    for t in (glob ($ROOT | path join examples vhs *.tape)) {
        let text = open --raw $t
        assert ($text | str contains "Set Shell \"nu\"") $t
        assert ($text | str contains "use examples/vhs/setup.nu demo-env; demo-env") $t
        # Nushell runs a relative path only with a leading ./
        assert ($text | str contains "Type \"./target/debug/") $t
    }
}

def "test gui capture: window bounds become a screencapture rectangle" [] {
    assert equal (rect "12, 40, 1200, 800") "12,40,1200,800"
}

def "test web capture: the browser screenshots the url at the size" [] {
    let a = shot-args /o/x.png 420 1100 "http://127.0.0.1:3978/history"
    assert ("--window-size=420,1100" in $a)
    assert ("--screenshot=/o/x.png" in $a)
    assert equal ($a | last) "http://127.0.0.1:3978/history"
}

def "test web capture: macOS defaults to Brave" [] {
    assert ((default-browser macos) | str ends-with "Brave Browser")
}

def main [] { run-tests }
