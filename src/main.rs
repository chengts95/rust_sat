//"https://satellitemap.space/json"

use std::{collections::HashMap, env};

use bevy::{
    asset::AssetPlugin,
    input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel},
    prelude::*,
    window::WindowResized,
};
use bevy_assets_bundler::{AssetBundlingOptions, BundledAssetIoPlugin};
use bevy_egui::{egui, EguiContext, EguiPlugin};
use bevy_prototype_lyon::prelude::ShapePlugin;
use rfd::AsyncFileDialog;

use bevy_svg::prelude::*;
mod celestrak;
mod render_satellite;
mod socket;
#[derive(Component)]
struct TaskWrapper<T>(Option<tokio::task::JoinHandle<T>>);

struct QueryConfig {
    timer: Timer,
}

struct RefreshConfig {
    timer: Timer,
}
struct CursorPosition(Vec2);
#[derive(Default)]
struct QueriedEvent;

#[derive(Default, Component)]
struct UIData(HashMap<String, String>);

use celestrak::*;
use render_satellite::*;

use tokio::runtime::Runtime;
#[derive(Default)]
struct SatConfigs {
    sat_color: Color,
    table_data: Vec<[String; 6]>,
    visible: Vec<Entity>,
}
fn show_data(
    mut egui_context: ResMut<EguiContext>,
    mut satcfg: ResMut<SatConfigs>,
    mut uidata: ResMut<UIData>,
    c: Res<CursorPosition>,
    rt: Res<Runtime>,
    sats: Query<(Entity, &SatID, &TEMEPos, &TEMEVelocity, &LatLonAlt, &Name)>,
    mut vis: Query<&mut Visibility, With<SatID>>,
) {
    egui::Window::new("Satellite Data").show(egui_context.ctx_mut(), |ui| {
        ui.label("Search Box:");
        let mut changed: bool = true;
        if let Some(text) = uidata.0.get_mut("searchbox") {
            let resp = ui.text_edit_singleline(text);

            changed = resp.changed();
        } else {
            uidata.0.insert(String::from("searchbox"), String::from(""));
        }

        ui.label(format!("{}", c.0));

        satcfg.visible.clear();
        let text = uidata.0.get("searchbox").unwrap();

        let filter = sats.iter().filter(|(e, _id, _pos, _vel, _lla, name)| {
            let name = name.to_string();
            let res = name.contains(text.as_str());
            if res {
                satcfg.visible.push(e.clone());
            }
            res
        });

        satcfg.table_data = filter
            .map(|(e, id, pos, vel, lla, name)| {
                let name = name.as_str();

                let a = [
                    e.id().to_string(),
                    id.0.to_string(),
                    name.to_string(),
                    format!("{:.2},{:.2},{:.2}", pos.0[0], pos.0[1], pos.0[2]),
                    format!("{:.2},{:.2},{:.2}", vel.0[0], vel.0[1], vel.0[2]),
                    format!("{:.2},{:.2},{:.2}", lla.0 .0, lla.0 .1, lla.0 .2),
                ];
                a
            })
            .collect();

        if ui.button("apply to map").clicked() {
            vis.for_each_mut(|mut y| {
                y.is_visible = false;
            });
            for i in &satcfg.visible {
                if let Ok(mut s) = vis.get_mut(*i) {
                    s.is_visible = true;
                }
            }
        }

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
                    for i in &satcfg.table_data {
                        for j in i {
                            f.write(j.as_bytes()).unwrap();
                            f.write(",".as_bytes()).unwrap();
                        }
                        f.write("\n".as_bytes()).unwrap();
                    }
                }
            }
        }
        create_table(ui, satcfg.table_data.iter());
    });
    config_ui(&mut egui_context, &mut satcfg);
}

fn config_ui(egui_context: &mut ResMut<EguiContext>, satcfg: &mut ResMut<SatConfigs>) {
    egui::Window::new("Configs").show(egui_context.ctx_mut(), |ui| {
        let a = satcfg.sat_color.clone();
        let mut srgba = unsafe {
            let ptr = (&mut a.as_rgba_u32() as *mut u32) as *mut u8;

            let srgba = egui::Color32::from_rgba_premultiplied(
                *ptr.offset(0),
                *ptr.offset(1),
                *ptr.offset(2),
                *ptr.offset(3),
            );
            srgba
        };
        ui.label("Satellite Color:");
        ui.color_edit_button_srgba(&mut srgba);
        let (r, g, b, a) = srgba.to_tuple();
        let srgba: [f32; 4] = [
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        ];
        satcfg.sat_color = Color::from(srgba);
    });
}

fn create_table<'a, T: ExactSizeIterator + Iterator<Item = &'a [String; 6]>>(
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
                for i in *a {
                    row.col(|ui| {
                        ui.text_edit_multiline(&mut i.as_str());
                    });
                }
            });
        });
}
fn main() {
    let mut app = App::new();
    let mut options = AssetBundlingOptions::default();

    options.encode_file_names = true;

    app.insert_resource(Msaa { samples: 4 });
    app.insert_resource(WindowDescriptor {
        title: "Satellite".to_string(),
        ..Default::default()
    });
    app.insert_resource(ClearColor(Color::rgb_u8(0, 7, 13)));
    app.insert_resource(SatConfigs {
        sat_color: Color::rgb_u8(0, 255, 202),
        ..Default::default()
    });
    app.add_plugins_with(DefaultPlugins, |group| {
        group.add_before::<AssetPlugin, _>(BundledAssetIoPlugin::from(options.clone()))
    })
    .add_plugin(EguiPlugin)
    .add_plugin(bevy_svg::prelude::SvgPlugin)
    .add_plugin(SatRenderPlugin)
    .add_plugin(ShapePlugin)
    .add_startup_system(setup);

    app.add_plugin(SGP4Plugin);
    app.insert_resource(UIData::default());

    app.add_system_to_stage(CoreStage::PreUpdate, cam_input_handle);
    app.add_system_to_stage(CoreStage::PostUpdate, show_data);
    app.add_system_to_stage(CoreStage::PreUpdate, resize_map);
    app.add_system_to_stage(CoreStage::PreUpdate, get_cursor_coord);

    app.run();
}

fn resize_map(
    mut svg: Query<(&Handle<Svg>, &mut Transform)>,
    svgs: Res<Assets<Svg>>,
    mut events: EventReader<WindowResized>,
) {
    for i in events.iter() {
        svg.for_each_mut(|(s, mut trans)| {
            let siz = svgs.get(s).unwrap().size;

            trans.scale.x = i.width / siz.x;
            trans.scale.y = i.height / siz.y;
        });
    }
}

fn cam_input_handle(
    scroll_evr: EventReader<MouseWheel>,
    mut ev_motion: EventReader<MouseMotion>,
    input_mouse: Res<Input<MouseButton>>,

    mut q: Query<(&mut OrthographicProjection, &mut Transform), With<Camera2d>>,
) {
    let mut acc = 0;
    scroll_handler(scroll_evr, &mut acc);

    q.for_each_mut(|(mut x, mut trans)| {
        let mut zoom = x.scale.ln();
        zoom += 0.1 * acc as f32;
        x.scale = zoom.exp();
        x.scale = x.scale.clamp(0.0, 1.2);
        if input_mouse.pressed(MouseButton::Middle) {
            for ev in ev_motion.iter() {
                trans.translation = trans.translation - Vec3::new(ev.delta.x, -ev.delta.y, 0.0);
            }
            if trans.translation.x < 0.0 {
                trans.translation.x = 0.0
            }
            if trans.translation.y > 0.0 {
                trans.translation.y = 0.0
            }
        }
    });
}

fn get_cursor_coord(
    mut commands: Commands,
    cc: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    wnds: Res<Windows>,
) {
    let (camera, camera_transform) = cc.single();
    for wnd in wnds.iter() {
        if let Some(screen_pos) = wnd.cursor_position() {
            let window_size = Vec2::new(wnd.width() as f32, wnd.height() as f32);

            // convert screen position [0..resolution] to ndc [-1..1] (gpu coordinates)
            let ndc = (screen_pos / window_size) * 2.0 - Vec2::ONE;
            let ndc_to_world =
                camera_transform.compute_matrix() * camera.projection_matrix().inverse();
            let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));
            // reduce it to a 2D value
            let world_pos: Vec2 = world_pos.truncate();
            commands.insert_resource(CursorPosition(world_pos));
        }
    }
}

fn scroll_handler(mut scroll_evr: EventReader<MouseWheel>, acc: &mut i32) {
    for ev in scroll_evr.iter() {
        match ev.unit {
            MouseScrollUnit::Line => {
                *acc += ev.y as i32;
            }
            MouseScrollUnit::Pixel => {
                *acc += ev.y as i32;
            }
        }
    }
}
fn setup(mut commands: Commands, asset_server: Res<AssetServer>, assets: Res<Assets<Svg>>) {
    let mut svg = asset_server.load("Mercator_Projection2.svg");
    let s = asset_server.load_folder("fonts").unwrap();
    for i in s {
        let h = i.typed::<Font>();
        commands.spawn().insert(h);
    }
    svg.make_strong(&assets);
    commands.insert_resource(CursorPosition(Vec2 { x: 0.0, y: 0.0 }));
    let mut camera = Camera2dBundle::default();
    //camera.projection.scaling_mode = ScalingMode::WindowSize;
    camera.transform.translation.x = 640.0;
    camera.transform.translation.y = -360.0;
    commands.spawn_bundle(camera);
    commands.spawn_bundle(Svg2dBundle {
        svg,
        origin: Origin::TopLeft,
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..Default::default()
    });
}
