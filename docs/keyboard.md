# Keyboard reference

This is Appendix A of the requirements (KEY-003, KEY-004). On macOS, use ⌘ wherever this table says Ctrl or Alt, except for History, which stays on Ctrl+H because ⌘H hides the app.

## All modes

| Key | Action |
|---|---|
| `0`–`9`, `.` or `,` | Digits and decimal point |
| `+` `-` `*` `/` | Operators |
| Enter, `=` | Equals |
| Esc | C (clear) |
| Delete | CE (clear entry) |
| Backspace | ← (delete last digit) |
| F9 | ± |
| `@` | √ |
| `r` | 1/x |
| Ctrl+M / R / L / P / Q | MS / MR / MC / M+ / M- (on macOS, ⌘Q quits) |
| Ctrl+C / Ctrl+V | Copy / paste |
| Alt+1 … Alt+4 (⌘1 … ⌘4) | Standard / Scientific / Programmer / Statistics |
| Ctrl+H | History on/off (Standard, Scientific) |
| Ctrl+Shift+D | Clear history |
| Ctrl+U / Ctrl+E / Ctrl+F4 | Unit conversion / date calculation / basic view |
| Ctrl+Shift+R | Reload themes |
| Ctrl++ / Ctrl+- / Ctrl+0 | Zoom in / zoom out / reset size |
| Ctrl+, | Preferences |
| Tab, Shift+Tab, arrow keys | Move between buttons |
| Space, Enter | Press the focused button |

## Standard

| Key | Action |
|---|---|
| `%` | Percent |

## Scientific

| Key | Action | Key | Action |
|---|---|---|---|
| `(` `)` | Parentheses | `i` | Inv |
| `s` `o` `t` | sin cos tan | `n` | ln (Inv: eˣ) |
| `l` | log | `g` | 10ˣ |
| `q` | x² (Inv: √x) | `#` | x³ (Inv: ∛x) |
| `y`, `^` | xʸ (Inv: ʸ√x) | `!` | n! |
| `%`, `d` | Mod | `;` | Int (Inv: Frac) |
| `m` | dms (Inv: deg) | `p` | π (Inv: 2π) |
| `x`, `e` | Exp | `v` | F-E |
| F3 / F4 / F5 | Degrees / radians / grads | | |

## Programmer

| Key | Action | Key | Action |
|---|---|---|---|
| `a`–`f` | Hex digits | `%` | Mod |
| `&` | And | `\|` | Or |
| `^` | Xor | `~` | Not |
| `<` or `<<` | Lsh | `>` or `>>` | Rsh |
| `j` | RoL | `k` | RoR |
| `(` `)` | Parentheses | | |
| F5 / F6 / F7 / F8 | Hex / Dec / Oct / Bin | | |
| F12 / F2 / F3 / F4 | Qword / Dword / Word / Byte | | |

Pasting a C expression such as `(1<<4)|3` works the same way.

## Statistics

| Key | Action |
|---|---|
| Insert, `a` | Add the entry to the list |
| C with a list item selected | Delete that item |

## Mouse

| Action | Result |
|---|---|
| Double-click the result | Copy it to the clipboard |
| Click a history entry | Load its result |
| Click a bit (Programmer) | Flip that bit |
| Click a statistics value, then C | Delete that value |
| Right-click a statistics value | Delete menu |
| Drag a window edge or corner | Zoom the whole calculator |
