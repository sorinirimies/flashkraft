#!/usr/bin/env nu
# Screenshots of the desktop GUI (macOS only — osascript + screencapture).
#
#   nu examples/capture/gui.nu <out-dir> <theme> <scene> [scene ...]
#
# Runs target/debug/flashkraft in demo mode (fake drives, simulated writes) and
# saves <out-dir>/gui-<scene>.png for each scene
# (home | ready | drives | batch | flashing | complete).

use ../vhs/setup.nu demo-env

const ROOT = path self ../..

# Ends job `id` and its process; a job that already finished is fine.
def stop [id: int] {
    if $id in (job list | get id) { job kill $id }
}

def osa [script: string]: nothing -> string {
    do { ^osascript -e $script } | complete | get stdout | str trim
}

# "x, y, w, h" from System Events as a screencapture -R rectangle.
export def rect [bounds: string]: nothing -> string {
    $bounds | split row "," | each { str trim } | str join ","
}

def main [out: string, theme: string, ...scenes: string] {
    if $nu.os-info.name != "macos" {
        print -e "gui.nu needs macOS (screencapture)"
        exit 1
    }
    mkdir $out
    demo-env $theme
    let process = {|pid| $"tell application \"System Events\" to tell \(first process whose unix id is ($pid)\)" }
    for scene in $scenes {
        let job = job spawn { with-env {FLASHKRAFT_DEMO_STEP_MS: "250", FLASHKRAFT_SCENE: $scene} { ^($ROOT | path join target debug flashkraft) o+e>| ignore } }
        sleep 4sec
        let pid = ^pgrep -n -f ($ROOT | path join target debug flashkraft) | str trim
        # The window must really be on top: screencapture grabs whatever is on
        # screen in that rectangle, so a covered window would capture another app.
        mut front = false
        for _ in 1..6 {
            osa $"(do $process $pid) to set frontmost to true" | ignore
            sleep 1sec
            if (osa $"(do $process $pid) to get frontmost") == "true" { $front = true; break }
        }
        if not $front {
            print -e $"skipping ($scene): could not bring the window to the front"
            stop $job
            continue
        }
        if $scene == "flashing" { sleep 3sec }
        sleep 1sec
        let r = rect (osa $"(do $process $pid) to get {position, size} of window 1")
        ^screencapture -x -R $r $"($out)/gui-($scene).png"
        stop $job
        print $"wrote ($out)/gui-($scene).png"
    }
}
