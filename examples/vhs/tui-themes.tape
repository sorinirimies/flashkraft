# VHS tape: cycle through colour themes (Ctrl+T) on the drive picker
Output docs/media/tui-themes.gif
Set Shell "bash"
Set FontSize 14
Set Width 1400
Set Height 850
Set Theme "TokyoNight"

Hide
Type "source examples/vhs/setup.sh && clear"
Enter
Type "target/debug/flashkraft-tui"
Enter
Sleep 2s
Type "/tmp/fk/debian-12.iso"
Enter
Sleep 1500ms
Down
Sleep 300ms
Type "m"
Down
Type "m"
Show
Sleep 1s
Ctrl+T
Sleep 1200ms
Ctrl+T
Sleep 1200ms
Ctrl+T
Sleep 1200ms
Ctrl+T
Sleep 1200ms
Ctrl+T
Sleep 1200ms
Escape
Escape
