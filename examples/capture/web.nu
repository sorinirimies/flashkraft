#!/usr/bin/env nu
# Screenshots of the web dashboard through headless Chromium/Brave.
#
#   nu examples/capture/web.nu <out-dir> [theme]
#
# Needs target/debug/flashkraft-web and the Topcoat bundle (`just bundle-web`).
# Runs in demo mode: fake drives, nothing is written. BROWSER=/path overrides
# the browser.

use ../vhs/setup.nu demo-env

const ROOT = path self ../..
const PORT = 3978
const IMAGE = "image=/tmp/fk/debian-12.iso"

# The first Chromium-based browser that exists.
export def default-browser [os: string]: nothing -> any {
    if $os == "macos" { return "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser" }
    [chromium google-chrome brave-browser] | each {|b| which $b | get -o path.0 } | compact | get -o 0
}

# Headless-browser arguments that screenshot `url` at width × height into `file`.
export def shot-args [file: string, width: int, height: int, url: string]: nothing -> list<string> {
    [--headless=new --disable-gpu --hide-scrollbars $"--window-size=($width),($height)" --virtual-time-budget=4000 $"--screenshot=($file)" $url]
}

# Ends job `id` and its process; a job that already finished is fine.
def stop [id: int] {
    if $id in (job list | get id) { job kill $id }
}

def serve [step_ms?: string] {
    let env_vars = {FLASHKRAFT_WEB_ASSETS: ($ROOT | path join target debug assets)} | merge (if $step_ms != null { {FLASHKRAFT_DEMO_STEP_MS: $step_ms} } else { {} })
    let job = job spawn { with-env $env_vars { ^($ROOT | path join target debug flashkraft-web) --port $PORT --no-open o+e>| ignore } }
    sleep 2sec
    $job
}

def shot [browser: string, out: string, name: string, width: int, height: int, path: string] {
    let file = $"($out)/($name).png"
    rm -f $file
    let b = job spawn { ^$browser ...(shot-args $file $width $height $"http://127.0.0.1:($PORT)/($path)") o+e>| ignore }
    # A stuck headless browser must not hang the script.
    for _ in 1..30 { if ($file | path exists) and (ls $file | get size.0) > 0b { break }; sleep 1sec }
    sleep 1sec
    stop $b
    if ($file | path exists) { print $"wrote ($file)" } else { print -e $"FAILED ($name)" }
}

def main [out: string, theme: string = "Tokyo Night"] {
    let browser = $env.BROWSER? | default (default-browser $nu.os-info.name)
    if $browser == null or not ($browser | path exists) {
        print -e "no Chromium-based browser found; set BROWSER=/path/to/it"
        exit 1
    }
    mkdir $out
    demo-env $theme
    let web = serve
    shot $browser $out web-wide 1400 760 $"?($IMAGE)"
    shot $browser $out web-medium 860 900 $"?($IMAGE)"
    shot $browser $out web-narrow 420 1100 $"?($IMAGE)"
    shot $browser $out web-history 1400 520 "history"
    stop $web
    # A flash in progress: three simulated drives, slowed down so it can be captured.
    let web = serve "200"
    let base = $"http://127.0.0.1:($PORT)"
    let token = http get --raw $"($base)/" | parse --regex '(?<t>[0-9a-f]{32})' | get t.0
    let body = {image: "/tmp/fk/debian-12.iso", devices: ["/dev/sdb" "/dev/sdc" "/dev/sdd"], confirm: 3}
    http post --content-type application/json -H [x-flashkraft-token $token] $"($base)/api/flash" $body | ignore
    sleep 4sec
    shot $browser $out web-flashing 1400 1150 $"?($IMAGE)"
    stop $web
}
