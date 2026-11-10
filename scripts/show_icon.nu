#!/usr/bin/env nu
# Show an icon (PNG) in the terminal: kitty graphics protocol (Ghostty, kitty, WezTerm),
# else ANSI half-blocks. Use it to judge the master at 256 px and at 32 px.
#
#   nu <skill>/scripts/show.nu assets/icons/<id>-256.png [--cells 24]

# Whether the terminal speaks the kitty graphics protocol.
export def kitty-capable [term: string, term_program: string, kitty_id: string]: nothing -> bool {
    ($term | str contains "ghostty") or ($term | str contains "kitty") or ($term_program == "WezTerm") or ($kitty_id != "")
}

# The escape sequences that draw `png` (base64) in 4096-byte chunks.
export def kitty-chunks [b64: string, cells: int]: nothing -> list<string> {
    let esc = (char --integer 27)
    let parts = $b64 | split chars | chunks 4096 | each { str join }
    $parts | enumerate | each {|c|
        let more = if $c.index < (($parts | length) - 1) { 1 } else { 0 }
        let head = if $c.index == 0 { $"f=100,a=T,c=($cells),m=($more)" } else { $"m=($more)" }
        $"($esc)_G($head);($c.item)($esc)\\"
    }
}

# Two pixels per character cell: upper half block, foreground = top, background = bottom.
export def half-blocks [rgb: binary, size: int]: nothing -> list<string> {
    let esc = (char --integer 27)
    let px = {|x, y| let i = ($y * $size + $x) * 3; 0..2 | each {|c| $rgb | bytes at ($i + $c)..($i + $c) | into int } }
    0..<($size // 2) | each {|row|
        let y = $row * 2
        0..<$size | each {|x|
            let t = do $px $x $y
            let b = do $px $x ($y + 1)
            $"($esc)[38;2;($t.0);($t.1);($t.2)m($esc)[48;2;($b.0);($b.1);($b.2)m▀"
        } | str join | $"($in)($esc)[0m"
    }
}

def main [png: path, --cells: int = 24] {
    if (kitty-capable ($env.TERM? | default "") ($env.TERM_PROGRAM? | default "") ($env.KITTY_WINDOW_ID? | default "")) {
        for c in (kitty-chunks (open --raw $png | encode base64) $cells) { print -n $c }
        print ""
    } else {
        let raw = ^magick $png -background "#0b0b12" -flatten -resize 40x40! -depth 8 rgb:- | into binary
        for l in (half-blocks $raw 40) { print $l }
    }
}
