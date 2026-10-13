# Deviations from the requirements specification

These are the places where the implementation deliberately differs from *Win7 Calculator Clone — Software Requirements Specification* (2026-10-13), and the requirement-level gaps that are still open. Each item should be copied into the specification as an amendment or closed.

## Amendments (decided)

| ID | Specification | Implementation and reason |
|---|---|---|
| ARC-001 | GUI toolkit: Slint or iced, decided by a spike | **egui/eframe**. It already provides AccessKit (UI-020), per-monitor DPI through winit (DPI-001/006/007), no redraws while idle (NFR-003) and wgpu rendering with a software-adapter fallback (ARC-003). The existing UI was built on it. |
| UI-002, UI-003, DPI-009 | Fixed-size window; zoom 75–200% through View ▸ Zoom | Dragging a window edge scales the whole UI uniformly from 75% to 400%. The proportions stay fixed, and the scale persists. `Ctrl +/-/0` and View ▸ Zoom in/out/Reset size also work. Mode switches keep the scale and anchor the window at its top-left corner. |
| STD-008 | Entries up to 32 significant digits | **16 in Standard and Statistics, 32 in Scientific.** This is what the Windows 7 reference does, and §3 says the reference wins. |
| THM-011 | An "Inter-class" fallback font | **Inter** 4.1, Regular and Bold (OFL 1.1). The previous Selawik font, a Microsoft font, was removed to satisfy NFR-040. |
| ENG-002 | Rational intermediate results (S) | Implemented as specified: parsing, `+ − × ÷`, `%`, `Mod`, integer powers and perfect roots stay exact. Very large fractions (over 70,000 bits) fall back to 256-bit binary floats to stay within ENG-009. |
| §10.3 | Open questions | Toolkit: egui. License: MIT. Product name: SevenPlus Calculator. EXT-004 Worksheets ship in 1.0. |

## Partially met (open)

| ID | State |
|---|---|
| HIS-002 (S) | Selecting a history entry loads its **result**, not its expression for editing. |
| DPI-008 (S) | Side panels do not yet move to a separate window when the screen is too small. |
| UI-006/007 | Window position is saved and restored on Windows, macOS and X11. Wayland does not let applications read or set window positions, so there the compositor places the window. |
| UI-024 (S) | High-contrast detection: Windows (SPI_GETHIGHCONTRAST), macOS ("Increase contrast") and GNOME (`a11y.interface high-contrast`). Other Linux desktops are not detected. |
| ARC-008 (S) | All UI text comes from Fluent (`assets/i18n/en-US/main.ftl`). Unit names, worksheet field names and engine error texts used outside the UI are still English constants in `calc-core`. |
| KEY-008 (S) | Num-pad keys work with NumLock on. With NumLock off, the OS sends navigation keys, which egui does not tell apart from the main block. |
| THM-010 (C) | `backdrop = "system"` is parsed. Native Mica/vibrancy is not implemented yet, so a solid background is used. |
| §10.1 golden tests | The runner and 150 sequences derived from the specification exist (`crates/calc-core/tests/golden/`). Recording the 2,000 reference sequences on the Windows 7 VM is still manual work. |
| §10.1 screenshot tests | CI captures screenshots of every mode on a software renderer (offline job artifact). Comparing them against approved baselines is not automated yet. |
| PKG-002/003/006 | Signing and notarization steps run when the certificate secrets are configured in the repository; see `.github/workflows/release.yml`. |
