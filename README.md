# SevenPlus

A Windows 7 calculator clone written in Rust, with the same functions and look. You can change the theme, and every colour, size and font is set in a TOML file.

Runs on Linux (.deb, .rpm), Windows and macOS.

## Features

- **Standard**: immediate execution (`2 + 3 * 4 = 20`), memory (MC MR MS M+ M-), `%`, `√`, `1/x`, `±`. Pressing `=` again repeats the last operation.
- **Scientific**: operator precedence (`2 + 3 * 4 = 14`) and parentheses. Also Inv, Degrees/Radians/Grads, trig and hyperbolic functions, `ln`, `log`, `xʸ`, `ʸ√x`, `n!` (gamma for non-integers), `dms`, `F-E`, `Exp`, `Mod`, `Int`/`Frac` and π. Results show 32 significant digits.
- **Programmer**: Hex, Dec, Oct and Bin bases with Qword, Dword, Word and Byte word sizes. Operators: And, Or, Xor, Not, Lsh, Rsh, RoL, RoR and Mod. The 64-bit binary panel has clickable bits.
- **Statistics**: a data list with x̄, x̄², Σx, Σx², σn and σn-1. Right-click a value to delete it.
- **History** (Ctrl+H), **Digit grouping**, **Copy/Paste**.
- **Unit conversion** (Ctrl+U), **Date calculation** (Ctrl+E), and **Worksheets**: mortgage, vehicle lease, fuel economy in mpg and L/100 km.
- Window frame: native OS frame, or a hand-drawn Windows 7 **Aero glass** frame.
- **Resizable**: drag a corner or edge and the whole UI scales uniformly, keeping the Win7 proportions. The scale is remembered (`scale` in `settings.toml`), and **View ▸ Reset size** returns to 100%. The Aero frame has its own resize grip in the bottom-right corner.

## Keyboard

| Key | Action | Key | Action |
|---|---|---|---|
| Alt+1…4 | Standard / Scientific / Programmer / Statistics | Ctrl+H | History |
| Enter, `=` | Equals | Esc | C |
| Del | CE | Backspace | ← |
| F9 | ± | `@` | √ |
| `r` | 1/x | `%` | % (Standard) / Mod |
| Ctrl+M / R / L / P / Q | MS / MR / MC / M+ / M- | Ctrl+C / Ctrl+V | Copy / Paste |
| `i` | Inv | `( )` | Parentheses |
| `s` `o` `t` | sin cos tan | `n` `l` `g` | ln log 10ˣ |
| `q` `#` `y` `!` | x² x³ xʸ n! | `p` `m` `v` `x` `;` | π dms F-E Exp Int |
| F3 / F4 / F5 | Degrees / Radians / Grads (Scientific) | | |
| F5–F8 | Hex / Dec / Oct / Bin (Programmer) | F12, F2–F4 | Qword / Dword / Word / Byte |
| `& \| ^ < > ~` | And Or Xor Lsh Rsh Not | `j` `k` | RoL RoR |
| Insert, `a` | Add (Statistics) | Ctrl+U / Ctrl+E / Ctrl+F4 | Units / Dates / Basic |

## Themes and settings

The settings live in the per-user config folder:

| OS | Folder |
|---|---|
| Linux | `~/.config/sevenplus/` |
| Windows | `%APPDATA%\sevenplus\config\` |
| macOS | `~/Library/Application Support/sevenplus/` |

- `settings.toml` holds the active theme, the window frame (`frame = "native"` or `"aero"`), the UI `scale`, the mode, history and other options. The app saves it automatically.
- `themes/*.toml` are the theme files. `win7.toml` and `win7-dark.toml` are written there on first run.

To make your own theme, copy `win7.toml` to a new file name and edit it. Then select it under **View ▸ Theme**. The same menu has **Open themes folder**.

The built-in Windows 7 theme is the base layer, so a theme only needs the keys it changes:

```toml
name = "My theme"

[button.hover]
gradient = ["#e0f0ff", "#a8d0f8"]

[button.equals]          # groups: digit, operator, memory, function, equals
gradient = ["#5d8fd6", "#2f63b0"]
text = "#ffffff"

[font]
file = "MyFont.ttf"      # relative to the themes folder
```

- **Colours** are `"#rrggbb"` or `"#rrggbbaa"`.
- **Gradients** are lists of colours spread evenly from top to bottom.
- **Sizes** are in logical pixels, covering button size, gap and corner radius, display height and font sizes, padding and title bar sizes.
- **Live reload**: changes to the active theme file apply while the app runs.
- **Errors**: a typo or invalid colour shows an error banner and the previous theme stays active.

`win7.toml` documents every key.

## Building

```sh
cargo run --release -p sevenplus
cargo test --workspace
```

On Debian/Ubuntu, install the GUI build dependencies first:

```sh
sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libgl1-mesa-dev
```

### Packages

| Target | Command | Output |
|---|---|---|
| .deb | `cargo build --release -p sevenplus && cargo deb -p sevenplus --no-build` | `target/debian/` |
| .rpm | `cargo build --release -p sevenplus && cargo generate-rpm -p crates/sevenplus` | `target/generate-rpm/` |
| macOS .app/.dmg | `cargo packager -p sevenplus --release --formats app,dmg` | `target/packages/` |
| Windows installer | `cargo packager -p sevenplus --release --formats nsis` | `target/packages/` |

Install the tools with `cargo install cargo-deb cargo-generate-rpm cargo-packager`.

Pushing a `v*` tag runs `.github/workflows/release.yml`. It builds the .deb, .rpm, a universal macOS .dmg, a Windows installer and a portable zip, then attaches them to a GitHub release.

## Project layout

- `crates/calc-core`: the calculator engine, with no GUI. It covers all modes, 256-bit floating-point arithmetic, Win7 number formatting, unit conversion, date calculation and worksheets, and is unit-tested.
- `crates/sevenplus`: the egui application. It contains the theme loader, the drawing code, the mode layouts and the extras panel.
- `assets/`: the built-in themes, the Selawik font and the icons.

Selawik is Microsoft's open-source substitute for Segoe UI, licensed under SIL OFL 1.1 (see `assets/fonts/Selawik-LICENSE.txt`). Segoe UI itself can't be redistributed. To use it on Windows, set `font.file` in a theme to `C:/Windows/Fonts/segoeui.ttf`.
