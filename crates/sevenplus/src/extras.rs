//! The right-hand extension panel: unit conversion, date calculation, worksheets.

use crate::config::Extras;
use calc_core::{
    Num, datecalc, units,
    worksheets::{self, Worksheet},
};
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

fn fmt(v: f64, grouping: bool) -> String {
    if v.is_finite() {
        Num::from_f64(v).format(16, false, grouping)
    } else {
        "Invalid input".into()
    }
}

fn parse(s: &str) -> Option<f64> {
    s.trim().replace(',', "").parse().ok()
}

fn readonly(ui: &mut Ui, text: &str, width: f32) {
    let mut t = text.to_string();
    ui.add(
        TextEdit::singleline(&mut t)
            .desired_width(width)
            .interactive(false),
    );
}

pub fn show(ui: &mut Ui, which: Extras, st: &mut ExtrasState, grouping: bool) {
    let w = ui.available_width();
    ui.spacing_mut().item_spacing.y = 6.0;
    match which {
        Extras::None => {}
        Extras::Units => units_ui(ui, st, w, grouping),
        Extras::Date => date_ui(ui, st, w),
        Extras::Mortgage => worksheet_ui(ui, st, 0, w, grouping),
        Extras::VehicleLease => worksheet_ui(ui, st, 1, w, grouping),
        Extras::FuelMpg => worksheet_ui(ui, st, 2, w, grouping),
        Extras::FuelL100km => worksheet_ui(ui, st, 3, w, grouping),
    }
}

fn units_ui(ui: &mut Ui, st: &mut ExtrasState, w: f32, grouping: bool) {
    let cats = units::CATEGORIES;
    ui.label("Select the type of unit you want to convert");
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
    ui.label("From");
    ui.add(TextEdit::singleline(&mut st.unit_input).desired_width(w));
    unit_combo(ui, "unit_from", &mut st.unit_from);
    ui.add_space(8.0);
    ui.label("To");
    let out = parse(&st.unit_input)
        .map(|v| {
            fmt(
                units::convert(st.unit_cat, st.unit_from, st.unit_to, v),
                grouping,
            )
        })
        .unwrap_or_default();
    readonly(ui, &out, w);
    unit_combo(ui, "unit_to", &mut st.unit_to);
}

fn date_ui(ui: &mut Ui, st: &mut ExtrasState, w: f32) {
    const MODES: [&str; 2] = [
        "Calculate the difference between two dates",
        "Add or subtract days to a specified date",
    ];
    ui.label("Select the date calculation you want");
    let mut mode = st.date_add_mode as usize;
    ComboBox::from_id_salt("date_mode")
        .width(w)
        .selected_text(MODES[mode])
        .show_ui(ui, |ui| {
            for (i, m) in MODES.iter().enumerate() {
                ui.selectable_value(&mut mode, i, *m);
            }
        });
    st.date_add_mode = mode == 1;
    ui.add_space(8.0);
    ui.label("From");
    date_picker(ui, &mut st.date_from, "date_from");
    if !st.date_add_mode {
        ui.label("To");
        date_picker(ui, &mut st.date_to, "date_to");
        let diff = datecalc::difference(st.date_from, st.date_to);
        ui.add_space(8.0);
        ui.label("Difference (years, months, weeks, days)");
        readonly(ui, &diff.describe(), w);
        ui.label("Difference (days)");
        let days = format!(
            "{} day{}",
            diff.total_days,
            if diff.total_days == 1 { "" } else { "s" }
        );
        readonly(ui, &days, w);
    } else {
        ui.horizontal(|ui| {
            ui.radio_value(&mut st.date_subtract, false, "Add");
            ui.radio_value(&mut st.date_subtract, true, "Subtract");
        });
        ui.horizontal(|ui| {
            for (label, v) in [
                ("Year(s)", &mut st.years),
                ("Month(s)", &mut st.months),
                ("Day(s)", &mut st.days),
            ] {
                ui.vertical(|ui| {
                    ui.label(label);
                    ui.add(egui::DragValue::new(v).range(0..=9999));
                });
            }
        });
        ui.add_space(8.0);
        ui.label("Date");
        let r = datecalc::add(st.date_from, st.years, st.months, st.days, st.date_subtract)
            .map_or_else(
                || "Date out of range".into(),
                |d| d.format("%A, %B %-d, %Y").to_string(),
            );
        readonly(ui, &r, w);
    }
}

fn worksheet_ui(ui: &mut Ui, st: &mut ExtrasState, i: usize, w: f32, grouping: bool) {
    let ws = worksheets::ALL[i];
    let fields = ws.fields();
    ui.label(RichText::new(ws.name()).strong());
    ui.label("Select the value you want to calculate");
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
    if ui.button("  Calculate  ").clicked() {
        let vals: Option<Vec<f64>> = st.ws_inputs[i]
            .iter()
            .enumerate()
            .map(|(j, s)| if j == target { Some(0.0) } else { parse(s) })
            .collect();
        st.ws_result[i] = Some(match vals.and_then(|v| ws.solve(target, &v)) {
            Some(r) => fmt(r, grouping),
            None => "Invalid input".into(),
        });
    }
    ui.add_space(4.0);
    ui.label(fields[target]);
    readonly(ui, st.ws_result[i].as_deref().unwrap_or(""), w);
}
