//! Keyboard, copy and paste (KEY-002 … KEY-008). The full map is in
//! `docs/keyboard.md` (Appendix A).

use crate::app::App;
use crate::menu::MenuCommand;
use calc_config::Extras;
use calc_core::keys::key_for_char;
use calc_core::{Angle, Base, Key, Mode, Num, WordSize};
use eframe::egui::{Context, Event, Key as EKey, Modifiers};

const MAC: bool = cfg!(target_os = "macos");

impl App {
    pub(crate) fn handle_keys(&mut self, ctx: &Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let (events, now, m) = ctx.input(|i| (i.events.clone(), i.time, i.modifiers));
        // A focused button is activated by Enter/Space through egui itself (UI-022).
        let focus = ctx.memory(|mem| mem.focused()).is_some();
        let mode_chord = if MAC { m.mac_cmd } else { m.alt };
        for ev in events {
            match ev {
                Event::Copy => ctx.copy_text(self.copy_text()),
                Event::Paste(s) => self.paste(&s),
                // The mode chords (Alt+1…4) must not also type their digit. Other text
                // with Alt held is kept: AltGr (Ctrl+Alt) types `@` on many layouts.
                Event::Text(t)
                    if !(mode_chord
                        && !(m.ctrl && m.alt)
                        && t.chars().all(|c| matches!(c, '1'..='4'))) =>
                {
                    for ch in t.chars() {
                        if let Some(key) = key_for_char(ch, self.calc.mode, self.calc.inv) {
                            self.press(key, now);
                        }
                    }
                }
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    if focus && matches!(key, EKey::Enter | EKey::Space) && !modifiers.any() {
                        continue;
                    }
                    self.key(ctx, key, modifiers, now);
                }
                _ => {}
            }
        }
    }

    fn key(&mut self, ctx: &Context, key: EKey, m: Modifiers, now: f64) {
        let mode = self.calc.mode;
        // KEY-002: Alt+1…4, or Cmd+1…4 on macOS.
        if (MAC && m.mac_cmd) || (!MAC && m.alt) {
            let target = match key {
                EKey::Num1 => Some(Mode::Standard),
                EKey::Num2 => Some(Mode::Scientific),
                EKey::Num3 => Some(Mode::Programmer),
                EKey::Num4 => Some(Mode::Statistics),
                _ => None,
            };
            if let Some(t) = target {
                self.set_mode(t);
                return;
            }
        }
        if m.command {
            let cmd = match key {
                EKey::H => Some(MenuCommand::ToggleHistory),
                EKey::U => Some(MenuCommand::Extras(Extras::Units)),
                EKey::E => Some(MenuCommand::Extras(Extras::Date)),
                EKey::F4 => Some(MenuCommand::Extras(Extras::None)),
                EKey::D if m.shift => Some(MenuCommand::ClearHistory),
                EKey::R if m.shift => Some(MenuCommand::ReloadThemes),
                EKey::Comma => Some(MenuCommand::Preferences),
                EKey::Plus | EKey::Equals => Some(MenuCommand::ZoomIn),
                EKey::Minus => Some(MenuCommand::ZoomOut),
                EKey::Num0 => Some(MenuCommand::ResetSize),
                _ => None,
            };
            if let Some(cmd) = cmd {
                self.command(ctx, cmd);
                return;
            }
            // KEY-003 memory keys. On macOS Cmd+Q stays Quit (PKG-004).
            let k = match key {
                EKey::M => Some(Key::MemStore),
                EKey::R => Some(Key::MemRecall),
                EKey::L => Some(Key::MemClear),
                EKey::P => Some(Key::MemAdd),
                EKey::Q if !MAC => Some(Key::MemSub),
                _ => None,
            };
            if let Some(k) = k {
                self.press(k, now);
            }
            return;
        }
        let k = match key {
            EKey::Enter => Some(Key::Equals),
            EKey::Escape => Some(Key::Clear),
            EKey::Delete => Some(Key::ClearEntry),
            EKey::Backspace => Some(Key::Back),
            EKey::F9 => Some(Key::Negate),
            EKey::Insert if mode == Mode::Statistics => Some(Key::StatAdd),
            _ => None,
        };
        if let Some(k) = k {
            self.press(k, now);
            return;
        }
        // KEY-004: angle units, radix and word size.
        match (mode, key) {
            (Mode::Scientific, EKey::F3) => self.calc.angle = Angle::Degrees,
            (Mode::Scientific, EKey::F4) => self.calc.angle = Angle::Radians,
            (Mode::Scientific, EKey::F5) => self.calc.angle = Angle::Gradians,
            (Mode::Programmer, EKey::F5) => self.calc.set_base(Base::Hex),
            (Mode::Programmer, EKey::F6) => self.calc.set_base(Base::Dec),
            (Mode::Programmer, EKey::F7) => self.calc.set_base(Base::Oct),
            (Mode::Programmer, EKey::F8) => self.calc.set_base(Base::Bin),
            (Mode::Programmer, EKey::F12) => self.calc.set_word(WordSize::Qword),
            (Mode::Programmer, EKey::F2) => self.calc.set_word(WordSize::Dword),
            (Mode::Programmer, EKey::F3) => self.calc.set_word(WordSize::Word),
            (Mode::Programmer, EKey::F4) => self.calc.set_word(WordSize::Byte),
            _ => {}
        }
    }

    /// KEY-007: a number is loaded directly; anything else is fed as keypresses and
    /// characters without a key are ignored.
    pub(crate) fn paste(&mut self, text: &str) {
        let text = text.trim();
        if self.calc.mode == Mode::Programmer {
            let digits: String = text.chars().filter(|c| !c.is_whitespace()).collect();
            let (neg, digits) = match digits.strip_prefix('-') {
                Some(d) => (true, d.to_string()),
                None => (false, digits),
            };
            if let Ok(v) = u64::from_str_radix(&digits, self.calc.base.radix()) {
                let n = Num::from_i128(v as i128);
                self.calc.load_value(if neg { n.neg() } else { n });
                return;
            }
        } else if let Some(n) = Num::parse(&calc_format::delocalize(text, &self.locale())) {
            self.calc.load_value(n);
            return;
        }
        let locale = self.locale();
        for ch in text.chars() {
            let ch = match ch {
                '×' => '*',
                '÷' => '/',
                '−' => '-',
                c if c == locale.decimal => '.',
                c => c,
            };
            if let Some(key) = key_for_char(ch, self.calc.mode, self.calc.inv) {
                self.calc.apply_key(key);
            }
        }
    }
}
