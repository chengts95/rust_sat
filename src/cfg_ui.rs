use bevy::prelude::*;
use bevy_egui::{
    egui::{self, Ui},
    EguiContexts, EguiPlugin, EguiSet,
};
use rfd::{AsyncFileDialog, FileHandle};

use std::{
    collections::HashMap,
    env,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::oneshot::{self, error::TryRecvError};

use crate::groundstation::GSConfigs;
use crate::*;

/// Stores the current cursor position as a Vec2.
#[derive(Resource)]
pub struct CursorPosition(pub Vec2);

/// Event triggered upon querying.
#[derive(Default)]
pub struct QueriedEvent;

/// Configuration resource for satellite settings.
#[derive(Default, Resource)]
pub struct SatConfigs {
    pub sat_color: Color,
    pub table_data: Vec<[String; 7]>,
    pub visible: Vec<Entity>,
    pub rx: Option<oneshot::Receiver<Option<FileHandle>>>,
}

/// UI-related data stored in JSON format.
#[derive(Default, Resource)]
pub struct UIData(serde_json::Value);

/// Component holding a map of UI strings.
#[derive(Default, Component)]
struct UIString(HashMap<String, String>);

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct EguiUISet;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct InputSet;

/// Main function to display data in the UI.
/// This function orchestrates different UI components, including the menu, configuration UI, and satellite data view.
pub fn show_data(
    mut egui_context: EguiContexts,
    mut satcfg: ResMut<SatConfigs>,
    mut gscfg: ResMut<GSConfigs>,
    mut cccfg: ResMut<ClearColor>,
    mut uidata: ResMut<UIData>,
    mut query: ResMut<QueryConfig>,
    c: Res<CursorPosition>,
    mut cam: Query<(&mut OrthographicProjection, &mut Transform)>,
    rt: Res<celestrak::Runtime>,
    sats: Query<(
        Entity,
        &SGP4Constants,
        &SatID,
        &TEMEPos,
        &TEMEVelocity,
        &LatLonAlt,
        &Name,
    )>,
    mut vis: Query<&mut Visibility, With<SatID>>,
) {
    show_menu(&mut egui_context, &mut uidata, &mut cam);
    show_config_ui(
        &mut egui_context,
        &mut satcfg,
        &mut gscfg,
        &mut cccfg,
        &mut uidata,
    );
    show_satellite_data(
        &mut egui_context,
        &mut uidata,
        &mut satcfg,
        &c,
        &sats,
        &mut vis,
        &mut query,
        &rt,
    );
}

/// Displays the configuration UI for various settings, including satellite and ground station colors.
fn show_config_ui(
    egui_context: &mut EguiContexts,
    satcfg: &mut ResMut<SatConfigs>,
    gscfg: &mut ResMut<GSConfigs>,
    cccfg: &mut ResMut<ClearColor>,
    uidata: &mut ResMut<UIData>,
) {
    let mut opened = uidata
        .0
        .get("Config")
        .unwrap_or(&false.into())
        .as_bool()
        .unwrap();
    config_ui(egui_context, satcfg, gscfg, cccfg, &mut opened);
    uidata.0["Config"] = opened.into();
}

/// Creates the top menu bar, including navigation and view controls.
fn show_menu(
    egui_context: &mut EguiContexts,
    uidata: &mut ResMut<UIData>,
    cam: &mut Query<(&mut OrthographicProjection, &mut Transform)>,
) {
    egui::TopBottomPanel::top("Menu").show(egui_context.ctx_mut(), |ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
            egui::menu::bar(ui, |ui| {
                let config_open = ui.menu_button("Config", |_ui| {}).response.clicked();
                let satellite_data_open = ui
                    .menu_button("Satellite Data", |_ui| {})
                    .response
                    .clicked();
                if config_open {
                    uidata.0["Config"] = config_open.into();
                }
                if satellite_data_open {
                    uidata.0["Satellite Data"] = satellite_data_open.into();
                }

                ui.menu_button("view", |ui| {
                    if ui.button("reset zoom").clicked() {
                        let (mut camera, _) = cam.single_mut();
                        camera.scale = 1.0;
                    }
                    if ui.button("center camera").clicked() {
                        let (_, mut camera) = cam.single_mut();
                        camera.translation = Vec3::new(512.0, 512.0, camera.translation.z);
                    }
                });
            });
        });
    });
}

/// Displays satellite data and provides controls to search, filter, and manage visibility of satellites.
fn show_satellite_data(
    egui_context: &mut EguiContexts,
    uidata: &mut ResMut<UIData>,
    satcfg: &mut ResMut<SatConfigs>,
    cursor: &Res<CursorPosition>,
    sats: &Query<(
        Entity,
        &SGP4Constants,
        &SatID,
        &TEMEPos,
        &TEMEVelocity,
        &LatLonAlt,
        &Name,
    )>,
    vis: &mut Query<&mut Visibility, With<SatID>>,
    query: &mut ResMut<QueryConfig>,
    rt: &Res<celestrak::Runtime>,
) {
    let mut opened = uidata
        .0
        .get("Satellite Data")
        .unwrap_or(&false.into())
        .as_bool()
        .unwrap();

    egui::Window::new("Satellite Data")
        .open(&mut opened)
        .show(egui_context.ctx_mut(), |ui| {
            ui.label(format!("{}", cursor.0));
            handle_search_box(ui, uidata, satcfg, sats);

            if ui.button("apply to map").clicked() {
                apply_visibility(vis, satcfg);
            }
            if ui.button("update TLE").clicked() {
                query.timer.reset();
            }
            #[cfg(not(target_arch = "wasm32"))]
            handle_export(ui, satcfg, rt);
            create_table(ui, satcfg.table_data.iter());
        });
    uidata.0["Satellite Data"] = opened.into();
}

/// Handles the export functionality by providing an option to save data to a CSV file asynchronously.
fn handle_export(ui: &mut Ui, satcfg: &mut SatConfigs, rt: &Res<Runtime>) {
    if ui.button("export").clicked() {
        let (tx, rx) = oneshot::channel();
        satcfg.rx = Some(rx);
        let _ = rt.0.spawn(async move {
            let file = AsyncFileDialog::new()
                .add_filter("csv", &["csv"])
                .set_directory(env::current_dir().unwrap().as_path())
                .save_file()
                .await;
            let _ = tx.send(file);
        });
    }
    if let Some(mut rx) = satcfg.rx.take() {
        match rx.try_recv() {
            Ok(f) => {
                if let Some(filename) = f {
                    let ts = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64();
                    use std::io::Write;
                    let mut f = std::fs::File::create(filename.path()).expect("create failed");

                    for i in &satcfg.table_data {
                        f.write(ts.to_string().as_bytes()).unwrap();
                        f.write(",".as_bytes()).unwrap();
                        for j in i {
                            f.write(j.as_bytes()).unwrap();
                            f.write(",".as_bytes()).unwrap();
                        }
                        f.write("\n".as_bytes()).unwrap();
                    }
                }
            }
            Err(TryRecvError::Empty) => {
                satcfg.rx = Some(rx);
            }
            _ => {}
        }
    }
}

/// Manages the search box functionality, allowing users to filter satellites based on name.
fn handle_search_box(
    ui: &mut egui::Ui,
    uidata: &mut ResMut<UIData>,
    satcfg: &mut ResMut<SatConfigs>,
    sats: &Query<(
        Entity,
        &SGP4Constants,
        &SatID,
        &TEMEPos,
        &TEMEVelocity,
        &LatLonAlt,
        &Name,
    )>,
) {
    ui.label("Search Box:");
    let mut text = uidata
        .0
        .get("searchbox")
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();
    text = text.trim_end().to_string();

    if ui.text_edit_singleline(&mut text).changed() {
        uidata.0["searchbox"] = text.clone().into();
    }

    satcfg.visible.clear();
    satcfg.table_data = sats
        .iter()
        .filter(|(e, _, _, _, _, _, name)| {
            let vis = name.contains(&text);
            if vis {
                satcfg.visible.push(e.clone());
            }
            vis
        })
        .map(|(e, elements, id, pos, vel, lla, name)| {
            format_satellite_data(e, elements, id, pos, vel, lla, name)
        })
        .collect();
}

/// Formats satellite data into a string array for display in the table.
fn format_satellite_data(
    e: Entity,
    elements: &SGP4Constants,
    id: &SatID,
    pos: &TEMEPos,
    vel: &TEMEVelocity,
    lla: &LatLonAlt,
    name: &Name,
) -> [String; 7] {
    let orbit: Orbit =
        serde_json::from_value(serde_json::to_value(&elements.0).unwrap()["orbit_0"].clone())
            .unwrap();
    [
        e.index().to_string(),
        id.0.to_string(),
        name.to_string(),
        format!("{:.2},{:.2},{:.2}", pos.0[0], pos.0[1], pos.0[2]),
        format!("{:.2},{:.2},{:.2}", vel.0[0], vel.0[1], vel.0[2]),
        format!("{:.2},{:.2},{:.2}", lla.0 .0, lla.0 .1, lla.0 .2),
        orbit.inclination.to_degrees().to_string(),
    ]
}

/// Sets satellite visibility based on the filtered satellite list.
fn apply_visibility(
    vis: &mut Query<'_, '_, &mut Visibility, With<SatID>>,
    satcfg: &ResMut<'_, SatConfigs>,
) {
    vis.iter_mut().for_each(|mut y| {
        *y = Visibility::Hidden;
    });
    for i in &satcfg.visible {
        if let Ok(mut s) = vis.get_mut(*i) {
            *s = Visibility::Visible;
        }
    }
}

/// Main configuration UI setup, enabling color adjustments for satellite, ground station, and clear color.
fn config_ui(
    egui_context: &mut EguiContexts,
    satcfg: &mut ResMut<SatConfigs>,
    gscfg: &mut ResMut<GSConfigs>,
    cccfg: &mut ResMut<ClearColor>,
    opened: &mut bool,
) {
    fn edit_color(ui: &mut egui::Ui, label: &str, color: &mut Color) {
        ui.label(label);
        let t = color.to_srgba();
        let mut srgba = egui::Color32::from_rgba_unmultiplied(
            (t.red * 255.0) as u8,
            (t.green * 255.0) as u8,
            (t.blue * 255.0) as u8,
            (t.alpha * 255.0) as u8,
        );

        if ui.color_edit_button_srgba(&mut srgba).changed() {
            let (red, green, blue, alpha) = srgba.to_tuple();
            *color = Color::srgba_u8(red, green, blue, alpha);
        }
    }

    egui::Window::new("Configs")
        .open(opened)
        .show(egui_context.ctx_mut(), |ui| {
            edit_color(ui, "Satellite Color:", &mut satcfg.sat_color);
            edit_color(ui, "Ground Station Color:", &mut gscfg.color);
            edit_color(ui, "Clear Color:", &mut cccfg.0);
        });
}

/// Generates a table for displaying satellite data.
/// Generates a table for displaying satellite data with specified column headers.
fn create_table<'a, T: ExactSizeIterator + Iterator<Item = &'a [String; 7]>>(
    ui: &mut egui::Ui,
    mut iter: T,
) {
    // Define headers and column generator to reduce duplicate code
    let headers = [
        "Entity ID", 
        "Norad ID", 
        "Name", 
        "TEME Coord", 
        "TEME Velocity", 
        "Latitude,Longitude,Altitude", 
        "Inclination"
    ];

    egui_extras::TableBuilder::new(ui)
        .columns(egui_extras::Column::remainder().resizable(true), headers.len())
        .header(50.0, |mut header| {
            for &title in &headers {
                header.col(|ui| { ui.heading(title); });
            }
        })
        .body(|body| {
            body.rows(30.0, iter.len(), |mut row| {
                let data = iter.next().unwrap();
                for cell in data {
                    row.col(|ui| {
                        ui.label(cell); // changed to `label` for simpler display
                    });
                }
            });
        });
}