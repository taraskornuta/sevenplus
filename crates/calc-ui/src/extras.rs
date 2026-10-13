//! The extension panel beside the keypad (EXT-005): unit conversion, date calculation
//! and worksheets.

use crate::i18n::{num, tr, tr_args};
use calc_config::Extras;
use calc_core::{
    Num, datecalc, units,
    worksheets::{self, Worksheet},
};
use calc_format::{FormatSettings, Locale, Value};
use chrono::{Datelike, Local, NaiveDate};
use eframe::egui::{self, ComboBox, RichText, TextEdit, Ui};
use egui_extras::DatePickerButton;

pub struct ExtrasState {
    unit_cat: usize,
    unit_from: usize,
    unit_to: usize,
    unit_input: String,
    date_add_mode: bool,
    date_from: NaiveDate,
    date_to: NaiveDate,
    date_subtract: bool,
    years: u32,
    months: u32,
    days: u32,
    ws_target: [usize; 4],
    ws_inputs: [Vec<String>; 4],
    ws_result: [Option<String>; 4],
}

impl Default for ExtrasState {
    fn default() -> Self {
        let today = Local::now().date_naive();
        let inputs = |ws: Worksheet| vec![String::new(); ws.fields().len()];
        ExtrasState {
            unit_cat: 3,
            unit_from: 0,
            unit_to: 1,
            unit_input: String::new(),
            date_add_mode: false,
            date_from: today,
            date_to: today,
            date_subtract: false,
            years: 0,
            months: 0,
            days: 0,
            ws_target: worksheets::ALL.map(|ws| ws.fields().len() - 1),
            ws_inputs: worksheets::ALL.map(inputs),
            ws_result: Default::default(),
        }
    }
}

fn to_jiff(d: NaiveDate) -> jiff::civil::Date {
    jiff::civil::Date::new(d.year() as i16, d.month() as i8, d.day() as i8).unwrap_or_default()
}

fn from_jiff(d: jiff::civil::Date) -> NaiveDate {
    NaiveDate::from_ymd_opt(d.year() as i32, d.month() as u32, d.day() as u32).unwrap_or_default()
}

fn date_picker(ui: &mut Ui, date: &mut NaiveDate, id: &'static str) {
    let mut j = to_jiff(*date);
    ui.add(
        DatePickerButton::new(&mut j)
            .id_salt(id)
            .calendar_week(false)
            .format("%A, %B %-d, %Y"),
    );
    *date = from_jiff(j);
}

fn fmt(v: f64, grouping: bool, locale: &Locale) -> String {
    if !v.is_finite() {
        return tr("error-invalid-input");
    }
    let s = FormatSettings {
        grouping,
        ..FormatSettings::default()
    };
    calc_format::format(&Value::Dec(Num::from_f64(v).to_decimal(24)), &s, locale)
}

fn parse(s: &str, locale: &Locale) -> Option<f64> {
    Num::parse(&calc_format::delocalize(s.trim(), locale)).map(|n| n.to_f64())
}

fn readonly(ui: &mut Ui, text: &str, width: f32) {
    let mut t = text.to_string();
    ui.add(
        TextEdit::singleline(&mut t)
            .desired_width(width)
            .interactive(false),
    );
}

pub fn show(ui: &mut Ui, which: Extras, st: &mut ExtrasState, grouping: bool, l: &Locale) {
    let w = ui.available_width();
    ui.spacing_mut().item_spacing.y = 6.0;
    match which {
        Extras::None => {}
        Extras::Units => units_ui(ui, st, w, grouping, l),
        Extras::Date => date_ui(ui, st, w),
        Extras::Mortgage => worksheet_ui(ui, st, 0, w, grouping, l),
        Extras::VehicleLease => worksheet_ui(ui, st, 1, w, grouping, l),
        Extras::FuelMpg => worksheet_ui(ui, st, 2, w, grouping, l),
        Extras::FuelL100km => worksheet_ui(ui, st, 3, w, grouping, l),
    }
}

fn units_ui(ui: &mut Ui, st: &mut ExtrasState, w: f32, grouping: bool, l: &Locale) {
    let cats = units::CATEGORIES;
    ui.label(tr("units-select"));
    let before = st.unit_cat;
    ComboBox::from_id_salt("unit_cat")
        .width(w)
        .selected_text(cats[st.unit_cat].name)
        .show_ui(ui, |ui| {
            for (i, c) in cats.iter().enumerate() {
                ui.selectable_value(&mut st.unit_cat, i, c.name);
            }
        });
    if before != st.unit_cat {
        st.unit_from = 0;
        st.unit_to = 1.min(cats[st.unit_cat].units.len() - 1);
    }
    let list = cats[st.unit_cat].units;
    let unit_combo = |ui: &mut Ui, id: &str, sel: &mut usize| {
        ComboBox::from_id_salt(id)
            .width(w)
            .selected_text(list[*sel].0)
            .show_ui(ui, |ui| {
                for (i, u) in list.iter().enumerate() {
                    ui.selectable_value(sel, i, u.0);
                }
            });
    };
    ui.add_space(8.0);
    ui.label(tr("units-from"));
    ui.add(TextEdit::singleline(&mut st.unit_input).desired_width(w));
    unit_combo(ui, "unit_from", &mut st.unit_from);
    ui.add_space(8.0);
    ui.label(tr("units-to"));
    let out = parse(&st.unit_input, l)
        .map(|v| {
            fmt(
                units::convert(st.unit_cat, st.unit_from, st.unit_to, v),
                grouping,
                l,
            )
        })
        .unwrap_or_default();
    readonly(ui, &out, w);
    unit_combo(ui, "unit_to", &mut st.unit_to);
}

fn date_ui(ui: &mut Ui, st: &mut ExtrasState, w: f32) {
    let modes = [tr("date-mode-difference"), tr("date-mode-add")];
    ui.label(tr("date-select"));
    let mut mode = st.date_add_mode as usize;
    ComboBox::from_id_salt("date_mode")
        .width(w)
        .selected_text(&modes[mode])
        .show_ui(ui, |ui| {
            for (i, m) in modes.iter().enumerate() {
                ui.selectable_value(&mut mode, i, m);
            }
        });
    st.date_add_mode = mode == 1;
    ui.add_space(8.0);
    ui.label(tr("date-from"));
    date_picker(ui, &mut st.date_from, "date_from");
    if !st.date_add_mode {
        ui.label(tr("date-to"));
        date_picker(ui, &mut st.date_to, "date_to");
        let diff = datecalc::difference(st.date_from, st.date_to);
        ui.add_space(8.0);
        ui.label(tr("date-difference-ymwd"));
        readonly(ui, &diff.describe(), w);
        ui.label(tr("date-difference-days"));
        let days = tr_args("date-days", &[("days", num(diff.total_days as f64))]);
        readonly(ui, &days, w);
    } else {
        ui.horizontal(|ui| {
            ui.radio_value(&mut st.date_subtract, false, tr("date-add"));
            ui.radio_value(&mut st.date_subtract, true, tr("date-subtract"));
        });
        ui.horizontal(|ui| {
            for (label, v) in [
                (tr("date-years"), &mut st.years),
                (tr("date-months"), &mut st.months),
                (tr("date-days-input"), &mut st.days),
            ] {
                ui.vertical(|ui| {
                    ui.label(label);
                    ui.add(egui::DragValue::new(v).range(0..=9999));
                });
            }
        });
        ui.add_space(8.0);
        ui.label(tr("date-result"));
        let r = datecalc::add(st.date_from, st.years, st.months, st.days, st.date_subtract)
            .map_or_else(
                || tr("date-out-of-range"),
                |d| d.format("%A, %B %-d, %Y").to_string(),
            );
        readonly(ui, &r, w);
    }
}

fn worksheet_ui(ui: &mut Ui, st: &mut ExtrasState, i: usize, w: f32, grouping: bool, l: &Locale) {
    let ws = worksheets::ALL[i];
    let fields = ws.fields();
    ui.label(RichText::new(ws.name()).strong());
    ui.label(tr("ws-select"));
    let before = st.ws_target[i];
    ComboBox::from_id_salt(("ws_target", i))
        .width(w)
        .selected_text(fields[st.ws_target[i]])
        .show_ui(ui, |ui| {
            for (j, f) in fields.iter().enumerate() {
                ui.selectable_value(&mut st.ws_target[i], j, *f);
            }
        });
    if before != st.ws_target[i] {
        st.ws_result[i] = None;
    }
    ui.add_space(8.0);
    let target = st.ws_target[i];
    for (j, f) in fields.iter().enumerate() {
        if j != target {
            ui.label(*f);
            ui.add(TextEdit::singleline(&mut st.ws_inputs[i][j]).desired_width(w));
        }
    }
    ui.add_space(4.0);
    if ui.button(format!("  {}  ", tr("ws-calculate"))).clicked() {
        let vals: Option<Vec<f64>> = st.ws_inputs[i]
            .iter()
            .enumerate()
            .map(|(j, s)| if j == target { Some(0.0) } else { parse(s, l) })
            .collect();
        st.ws_result[i] = Some(match vals.and_then(|v| ws.solve(target, &v)) {
            Some(r) => fmt(r, grouping, l),
            None => tr("error-invalid-input"),
        });
    }
    ui.add_space(4.0);
    ui.label(fields[target]);
    readonly(ui, st.ws_result[i].as_deref().unwrap_or(""), w);
}
