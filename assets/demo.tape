# The README's Notes hold the full regeneration steps, including the crop.

Output target/vhs/demo.gif

Set FontSize 8
Set Width 1280
Set Height 800
Set Padding 0
Set Framerate 1
Set WaitTimeout 10m

Hide
Type "cargo build --release -q"
Enter
Wait
Type "target/release/plurimus-doom -iwad freedoom1.wad"
Enter
Show

Wait+Screen /KEYS/
Sleep 12s
Screenshot assets/screenshot.png
