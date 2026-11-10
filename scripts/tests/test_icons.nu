#!/usr/bin/env nu
# ── FlashKraft · test_icons.nu ────────────────────────────────────────
# scripts/icons.nu: the variants of the master artwork and the files it writes.

use std/assert
use runner.nu *
use ../project.nu [APP]
use ../icons.nu [MASTER SMALL_MAX ICO_SIZES GUI_ASSETS WINDOW_RGBA WINDOW_ICON_SIZE variant kind-for iconset-entries outputs ICNS_TYPES u32-be icns-entry icns-bytes pick-rasteriser raster-args]
use ../ci/package_desktop.nu [ICON_SIZES]

const ROOT = path self ../..

def master [] { open --raw ($ROOT | path join $MASTER) }

def "test icons: the master marks the blocks the variants edit" [] {
    let m = master
    for b in [construction mark rim] {
        assert ($m | str contains $"<!-- ($b):start -->") $"($b) start marker"
        assert ($m | str contains $"<!-- ($b):end -->") $"($b) end marker"
    }
}

def "test icons: full is the master unchanged" [] {
    assert equal (variant (master) full) (master)
}

def "test icons: small drops construction, detail and rim and enlarges the mark" [] {
    let s = variant (master) small
    for b in [construction detail rim] { assert not ($s | str contains $"($b):start") $"no ($b) block" }
    assert not ($s | str contains "stroke-dasharray") "the dashed guides are gone"
    assert ($s | str contains "<!-- mark:start --><g transform=\"translate(512 512) scale(1.12) translate(-512 -512)\">")
    assert ($s | str contains "</g><!-- mark:end -->")
    assert ($s | str ends-with "</svg>\n")
}

def "test icons: macos puts the art on the apple icon grid" [] {
    let m = variant (master) macos
    assert ($m | str contains 'translate(100 100) scale(0.8046875)') "824 px body, centred"
    assert ($m | str contains 'filter="url(#mac-shadow)"')
    assert equal ($m | split row "<svg" | length) 2 "one svg element, the master's is unwrapped"
    assert ($m | str contains "construction:start")
    assert not (variant (master) macos-small | str contains "construction:start")
}

def "test icons: an unknown variant is an error" [] {
    assert error { variant (master) huge }
}

def "test icons: small sizes use the small variant" [] {
    assert equal (kind-for 16 false) small
    assert equal (kind-for $SMALL_MAX false) small
    assert equal (kind-for 48 false) full
    assert equal (kind-for 32 true) macos-small
    assert equal (kind-for 1024 true) macos
}

def "test icons: the iconset covers 16 to 1024 px" [] {
    let e = iconset-entries
    assert equal ($e | length) 10
    assert equal ($e | where name == "icon_512x512@2x.png" | get px) [1024]
    assert equal ($e | where name == "icon_16x16.png" | get px) [16]
}

def "test icons: every generated file is committed" [] {
    for f in (outputs) { assert ($ROOT | path join $f | path exists) $"($f) exists: run `just icons`" }
    for n in $ICON_SIZES { assert (($"assets/icons/($APP.id)-($n).png") in (outputs)) $"($n) px png is generated" }
}

def "test icons: the window icon is raw rgba of its size" [] {
    if not $WINDOW_RGBA { return }
    let f = $ROOT | path join $GUI_ASSETS $"($APP.id)-($WINDOW_ICON_SIZE).rgba"
    assert equal (ls $f | get size.0 | into int) ($WINDOW_ICON_SIZE * $WINDOW_ICON_SIZE * 4)
}

def "test icons: the ico entries are bmp, which nsis needs" [] {
    let ico = open --raw ($ROOT | path join packaging windows $"($APP.id).ico") | into binary
    assert equal ($ico | bytes at 0..3) 0x[00000100] "an ico header"
    let n = $ico | bytes at 4..5 | into int --endian little
    assert equal $n ($ICO_SIZES | length)
    for i in 0..<$n {
        let at = 6 + 16 * $i
        let offset = $ico | bytes at ($at + 12)..($at + 15) | into int --endian little
        assert equal ($ico | bytes at $offset..($offset + 3)) 0x[28000000] $"entry ($i) starts with a BITMAPINFOHEADER"
    }
}

def "test icons: u32-be writes four big-endian bytes" [] {
    assert equal (u32-be 258) 0x[00 00 01 02]
}

def "test icons: an icns entry and the file carry their lengths" [] {
    let e = icns-entry icp4 0x[01 02 03]
    assert equal $e 0x[69637034 0000000B 010203]
    assert equal (icns-bytes [$e]) 0x[69636E73 00000013 69637034 0000000B 010203]
}

def "test icons: the icns covers 16 to 1024 px with retina pairs" [] {
    assert equal ($ICNS_TYPES | length) 11
    assert equal ($ICNS_TYPES | get px | math max) 1024
}

def "test icons: the committed icns is a valid icns with a png in every entry" [] {
    let b = open --raw ($ROOT | path join packaging macos $"($APP.id).icns") | into binary
    assert equal ($b | bytes at 0..3 | decode utf-8) icns
    let total = $b | bytes at 4..7 | into int --endian big
    assert equal ($b | bytes length) $total
    mut at = 8
    mut n = 0
    while $at < $total {
        let len = $b | bytes at ($at + 4)..($at + 7) | into int --endian big
        assert equal ($b | bytes at ($at + 8)..($at + 11)) 0x[89504E47] "a PNG signature"
        $at += $len
        $n += 1
    }
    assert equal $n ($ICNS_TYPES | length)
}

def "test icons: either rasteriser works" [] {
    assert equal (pick-rasteriser [rsvg-convert]) rsvg-convert
    assert equal (pick-rasteriser [resvg rsvg-convert]) resvg
    assert equal (pick-rasteriser []) null
    assert equal (raster-args rsvg-convert a.svg 32 o.png) ["-w" "32" "-h" "32" a.svg "-o" o.png]
}

def main [] { run-tests }
