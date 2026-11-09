# Regenerates assets/screenshot.png; see the README's Notes for the crop
# that follows. Run from the repo root after ./fetch-wad.sh.

Output target/vhs/demo.gif

Set Shell bash
Set FontSize 8
Set Width 1280
Set Height 800
Set Padding 0
Set WaitTimeout 10m

Hide
Type "cargo build --release -q && clear && echo $((6*7))built"
Enter
Wait+Screen /42built/
Type "target/release/plurimus-doom -iwad freedoom1.wad"
Enter
Show

Sleep 12s
Screenshot assets/screenshot.png
