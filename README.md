# plurimus-doom

Doom running in the terminal, rendered as colored halfblock cells (`▀`/`▄`, two
pixels per cell) through [plurimus], the Bevy-native terminal renderer. The
engine is [doomgeneric] driven from a dedicated thread; every frame flows
through plurimus's camera/compositor pipeline like any other Bevy app.

[plurimus]: https://github.com/kenianbei/plurimus
[doomgeneric]: https://github.com/ozkl/doomgeneric

## Requirements

- Rust 1.95+ and a C compiler (doomgeneric's C sources build via `cc`).
- A kitty-keyboard-protocol terminal (kitty, foot, WezTerm, recent
  Ghostty/Alacritty) for real key press/release events. Legacy terminals work,
  but held-key movement relies on synthesized releases and feels jerky.
- Linux (fd redirection and termios restore use libc directly).

## Running

```sh
./fetch-wad.sh        # downloads freedoom1.wad (free, BSD-licensed)
cargo run --release
```

The default build blits frames on the CPU. `cargo run --release --features gpu`
renders the frame on a CRT-shaded quad through plurimus's 3d pipeline instead,
with `t` cycling the pixel-to-cell strategy (halfblocks, shading and ascii
luminance ramps, braille) and `g` toggling a sobel edge overlay.

Any IWAD works: pass `-iwad <path>`, or drop `doom1.wad`, `doom.wad`,
`freedoom1.wad`, or `freedoom2.wad` in the working directory.

## Controls

| key                     | action                                    |
| ----------------------- | ----------------------------------------- |
| arrows                  | move and turn                             |
| shift + left/right      | strafe                                    |
| wasd                    | move forward/back + strafe left/right     |
| space                   | fire                                      |
| e                       | use                                       |
| r                       | run                                       |
| Esc, Enter, digits, y/n | menus, weapons, prompts                   |
| t                       | cycle the render strategy (gpu build)     |
| g                       | toggle the sobel edge overlay (gpu build) |
| ctrl-c                  | quit                                      |

Quitting through Doom's own menu also restores the terminal (an `atexit` hook
covers the engine's direct `exit()` call).

## Notes

- The terminal is driven through `/dev/tty` while stdout and stderr point
  permanently at `doom.log` — all engine output (startup banner, warnings,
  in-game prints) lands there. Check the log if the game exits immediately
  (usually a WAD problem).
- The engine renders at 320x200 (set at build time via `DOOMGENERIC_RESX/RESY`
  in `.cargo/config.toml`); the blit letterboxes it into whatever size your
  terminal is.

## License

GPL-2.0-only, following the doomgeneric engine sources. plurimus is consumed
under the MIT option of its MIT OR Apache-2.0 license.
