# VHS tape: select image → mark three drives → flash them all at once (demo mode, simulated writes)
Output docs/media/tui-workflow.gif
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
Show
Sleep 1s
Type "/tmp/fk/debian-12.iso"
Sleep 800ms
Screenshot docs/media/tui-select-image.png
Enter
Sleep 1500ms
Down
Sleep 400ms
Type "m"
Sleep 400ms
Down
Sleep 400ms
Type "m"
Sleep 400ms
Down
Sleep 400ms
Type "m"
Sleep 1s
Screenshot docs/media/tui-select-drive.png
Enter
Sleep 1s
Screenshot docs/media/tui-drive-info.png
Type "f"
Sleep 1500ms
Screenshot docs/media/tui-confirm.png
Type "y"
Sleep 3s
Screenshot docs/media/tui-flashing.png
Sleep 9s
Screenshot docs/media/tui-complete.png
Sleep 2s
Type "q"
Sleep 500ms
