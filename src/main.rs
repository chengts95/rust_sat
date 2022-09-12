//"https://satellitemap.space/json"

use std::{collections::HashMap, env};

use bevy::prelude::*;
use bevy_egui::{egui, EguiContext, EguiPlugin};
use rfd::AsyncFileDialog;

use std::io::{Write};

use bevy_svg::prelude::*;
mod celestrak;
mod socket;

#[derive(Component)]
struct TaskWrapper<T>(Option<tokio::task::JoinHandle<T>>);

struct QueryConfig {
    timer: Timer,
}

struct RefreshConfig {
    timer: Timer,
}

#[derive(Default)]
struct QueriedEvent;

#[derive(Default, Component)]
struct UIData(HashMap<String, String>);

use celestrak::*;
use tokio::runtime::Runtime;

fn show_data(
    mut egui_context: ResMut<EguiContext>,
    mut uidata: ResMut<UIData>,
    data: Res<SatInfo>,
    rt: Res<Runtime>,
    sats: Query<(Entity, &SatID, &TEMEPos, &TEMEVelocity, &LatLonAlt)>,
) {
    egui::Window::new("Show Plots").show(egui_context.ctx_mut(), |ui| {
        ui.label("Search Box:");

        if let Some(text) = uidata.0.get_mut("searchbox") {
            ui.text_edit_singleline(text);
        } else {
            uidata.0.insert(String::from("searchbox"), String::from(""));
        }
        let text = uidata.0.get("searchbox").unwrap();

        let iter: Vec<_> = sats
            .iter()
            .filter(|(_e, id, _pos, _vel, _lla)| {
                let name = data
                    .sats
                    .get(&id.0)
                    .unwrap()
                    .object_name
                    .as_ref()
                    .unwrap()
                    .to_owned();
                name.contains(text.as_str())
            })
            .map(|(e, id, pos, vel, lla)| {
                let name = data
                    .sats
                    .get(&id.0)
                    .unwrap()
                    .object_name
                    .as_ref()
                    .unwrap()
                    .to_owned();

                let a = [
                    e.id().to_string(),
                    id.0.to_string(),
                    name,
                    format!("{:.2},{:.2},{:.2}", pos.0[0], pos.0[1], pos.0[2]),
                    format!("{:.2},{:.2},{:.2}", vel.0[0], vel.0[1], vel.0[2]),
                    format!("{:.2},{:.2},{:.2}", lla.0 .0, lla.0 .1, lla.0 .2),
                ];
                a
            })
            .collect();
        #[cfg(not(target_arch = "wasm32"))]
        {
            if ui.button("export").clicked() {
                let f = rt.block_on(async {
                    let file = AsyncFileDialog::new()
                        .add_filter("csv", &["csv"])
                        .set_directory(env::current_dir().unwrap().as_path())
                        .save_file()
                        .await;
                    file
                });
                if let Some(filename) = f {
                    use std::io::Write;
                    let mut f = std::fs::File::create(filename.path()).expect("create failed");
                    for i in &iter {
                        for j in i {
                            f.write(j.as_bytes()).unwrap();
                            f.write(",".as_bytes()).unwrap();
                        }
                        f.write("\n".as_bytes()).unwrap();
                    }
                }
            }
        }
        create_table(ui, iter.into_iter());
    });
}


fn create_table<'a, T: ExactSizeIterator + Iterator<Item = [String; 6]>>(
    ui: &mut egui::Ui,
    iter: T,
) {
    let v: Vec<_> = iter.collect();
    let _tb = egui_extras::TableBuilder::new(ui)
        .columns(egui_extras::Size::remainder().at_least(50.0), 6)
        .header(50.0, |mut header| {
            header.col(|ui| {
                ui.heading("Entity ID");
            });

            header.col(|ui| {
                ui.heading("Norad ID");
            });
            header.col(|ui| {
                ui.heading("Name");
            });
            header.col(|ui| {
                ui.heading("TEME Coord");
            });

            header.col(|ui| {
                ui.heading("TEME Velocity");
            });
            header.col(|ui| {
                ui.heading("Latitude,Longitude,Altitude");
            });
        })
        .body(|body| {
            body.rows(30.0, v.len(), |row_index, mut row| {
                let a = &v[row_index];
                for i in a {
                    row.col(|ui| {
                        ui.text_edit_multiline(&mut i.as_str());
                    });
                }
            });
        });
}
fn main() {

    let mut app = App::new();

    app.insert_resource(Msaa { samples: 4 });
    app.add_plugins(DefaultPlugins).add_plugin(EguiPlugin)
    .add_plugin(bevy_svg::prelude::SvgPlugin)
    .add_startup_system(setup);
    app.add_plugin(SGP4Plugin);
    app.insert_resource(UIData::default());


    app.add_system_to_stage(CoreStage::PostUpdate, show_data);
    app.run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let svg = asset_server.load("assets/world.svg");
    commands.spawn_bundle(Camera2dBundle::new_with_far(0.1));
    commands.spawn_bundle(Svg2dBundle {
        svg,
        origin: Origin::Center, // Origin::TopLeft is the default
        ..Default::default()
    });
}