# Themes

A theme is a TOML file of design tokens. Every built-in theme is a file in [`assets/themes/`](../assets/themes). [`aero7.toml`](../assets/themes/aero7.toml) is the base of every other theme. It lists every token and has comments explaining each one. Older themes may still have a `[titlebar]` table from the removed Aero glass frame; it is ignored.

## Making a theme

1. Open **View ▸ Themes ▸ Open themes folder**.
2. Create a file such as `ocean.toml`:

   ```toml
   schema = 1
   name = "Ocean"
   extends = "dark"          # optional; without it the theme builds on aero7

   [display]
   background = ["#0b2a3a", "#06202d"]
   text = "#e0f7ff"

   [button.equals.normal]
   fill = ["#1aa3c7", "#0b7fa0"]
   text = "#ffffff"
   ```

3. Choose **Ocean** in **View ▸ Themes**. When you edit the file, the calculator reloads it the next time its window gets focus. `Ctrl+Shift+R` rescans the folder.

A theme only needs the tokens it changes. Everything else comes from the theme it `extends` (THM-006). A user file can also replace a built-in theme by using the same name, e.g. `dark.toml`. `extends = "dark"` inside it then refers to the built-in Dark theme.

## Tokens

| Table | Tokens |
|---|---|
| top level | `schema` (must be 1), `name`, `extends`, `backdrop` (`"solid"` or `"system"`) |
| `[font]` | `file`, `bold_file`: font paths, relative to the themes folder. Empty uses the bundled Inter. |
| `[font.<role>]` | `size` (logical px), `weight` (`"regular"` or `"bold"`). Roles: `ui`, `button`, `display`, `expression`, `indicator`, `history`, `bits`, `caption` |
| `[spacing]` | `padding`, `gap`, `button_width`, `button_height` |
| `[window]` | `background` |
| `[menu]` | `height`, `background`, `text`, `hover`, `hover_border`, `popup_background`, `popup_border` |
| `[display]` | `background`, `border`, `radius`, `height`, `padding`, `text`, `expression`, `indicator`, `history_text`, `history_hover`, `history_lines`, `selection`, `selection_text` |
| `[button]` | `radius` |
| `[button.<group>.<state>]` | `fill`, `border`, `text`, `highlight`, `radius` |
| `[controls]` | `text`, `background`, `border`, `accent`, `group_border`, `hover`, `focus` |
| `[bits]` | `text`, `dim`, `index` |
| `[imagery]` | `window`: an SVG drawn over the window background |

The button groups are `digit`, `operator`, `memory`, `function` and `equals`, plus `default`, which applies to all of them. The states are `normal`, `hover`, `pressed`, `disabled` and `focus`.

- **Colors** are `"#rrggbb"` or `"#rrggbbaa"`.
- A `background` or `fill` is a single color or a list of 2–4 colors that blend from top to bottom.
- **Sizes** are logical pixels and scale with the display automatically. A theme cannot set physical pixels.

## Validation

Theme files must be at most 1 MB. Unknown keys, invalid colors, gradients with more than four colors, out-of-range sizes, a missing or newer `schema`, and `extends` cycles are all errors. A theme with an error is not used: the calculator falls back to the default theme, shows the error at the bottom of the window and writes it to the log.

All built-in themes meet WCAG 2.1 AA contrast (4.5:1) for button labels and display text, and a unit test checks this. If you share a theme, aim for the same.
