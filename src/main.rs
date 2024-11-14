//"https://satellitemap.space/json"

use bevy::{
    color::palettes::css::YELLOW, input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel}, prelude::*, render::view::NoFrustumCulling, sprite::Mesh2dHandle, window::PrimaryWindow
};
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiSet};
use bevy_embedded_assets::EmbeddedAssetPlugin;
use bevy_prototype_lyon::prelude::ShapePlugin;


use datalink::{DatalinkPlugin, GSDataLink};

use groundstation::{GSConfigs, GSPlugin, GroundStationBundle, GroundStationID};
use rfd::{AsyncFileDialog, FileHandle};
use sgp4::Orbit;
use std::{
    collections::HashMap,
    env,
    future::IntoFuture,
    time::{self, SystemTime, UNIX_EPOCH},
};
use tokio::{
    sync::oneshot::{self, error::TryRecvError},
    task::JoinHandle,
};

use bevy_svg::prelude::*;
pub mod celestrak;
mod datalink;
pub mod groundstation;
pub mod render_satellite;

pub mod util;
#[cfg(feature = "zmq_comm")]
pub mod zmq_comm;
// #[derive(Resource)]
// struct RefreshConfig {
//     timer: Timer,
// }
#[derive(Resource)]
struct CursorPosition(Vec2);
#[derive(Default)]
struct QueriedEvent;

#[derive(Default, Resource)]
struct UIData(serde_json::Value);
#[derive(Default, Component)]
struct UIString(HashMap<String, String>);
use celestrak::*;
use render_satellite::*;

#[derive(Default, Resource)]
struct SatConfigs {
    sat_color: Color,
    table_data: Vec<[String; 7]>,
    visible: Vec<Entity>,
    rx: Option<oneshot::Receiver<Option<FileHandle>>>,
}
fn show_data(
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
    egui::TopBottomPanel::top("Menu").show(egui_context.ctx_mut(), |ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
            egui::menu::bar(ui, |ui| {
                let a = ui.menu_button("Config", |_ui| {}).response.clicked();
                let b = ui
                    .menu_button("Satellite Data", |_ui| {})
                    .response
                    .clicked();
                if a {
                    uidata.0["Config"] = a.into();
                }
                if b {
                    uidata.0["Satellite Data"] = b.into();
                }

                ui.menu_button("view", |ui| {
                    if ui.button("reset zoom").clicked() {
                        let (mut camera, _) = cam.single_mut();
                        camera.scale = 1024.0;
                    }
                    if ui.button("center camera").clicked() {
                        let (_, mut camera) = cam.single_mut();

                        camera.translation.x = 512.0;
                        camera.translation.y = 512.0;
                    }
                });
            });
        });
    });
    let mut opened = uidata
        .0
        .get("Config")
        .unwrap_or(&false.into())
        .as_bool()
        .unwrap();
    config_ui(
        &mut egui_context,
        &mut satcfg,
        &mut gscfg,
        &mut cccfg,
        &mut opened,
    );
    uidata.0["Config"] = opened.into();
    let mut opened = uidata
        .0
        .get("Satellite Data")
        .unwrap_or(&false.into())
        .as_bool()
        .unwrap();

    egui::Window::new("Satellite Data")
        .open(&mut opened)
        .show(egui_context.ctx_mut(), |ui| {
            ui.label(format!("{}", c.0));
            ui.label("Search Box:");
            let mut text = String::from("");
            if !uidata.0["searchbox"].is_null() {
                text = uidata.0["searchbox"].as_str().unwrap().to_string();
                text = text.strip_suffix(" ").unwrap_or(text.as_str()).to_string();
            }
            let res = ui.text_edit_singleline(&mut text).changed();
            if res {
                uidata.0["searchbox"] = text.clone().into();
            }

            satcfg.visible.clear();

            let filter = sats
                .iter()
                .filter(|(e, _elements, _id, _pos, _vel, _lla, name)| {
                    let name = name.to_string();
                    let res = name.contains(&text);
                    if res {
                        satcfg.visible.push(e.clone());
                    }
                    res
                });

            satcfg.table_data = filter
                .map(|(e, elements, id, pos, vel, lla, name)| {
                    let name = name.as_str();
                    let element = serde_json::to_value(elements.0.clone()).unwrap();
                    let orbit: Orbit = serde_json::from_value(element["orbit_0"].clone()).unwrap();
                    let a = [
                        e.index().to_string(),
                        id.0.to_string(),
                        name.to_string(),
                        format!("{:.2},{:.2},{:.2}", pos.0[0], pos.0[1], pos.0[2]),
                        format!("{:.2},{:.2},{:.2}", vel.0[0], vel.0[1], vel.0[2]),
                        format!("{:.2},{:.2},{:.2}", lla.0 .0, lla.0 .1, lla.0 .2),
                        orbit.inclination.to_degrees().to_string(),
                    ];
                    a
                })
                .collect();

            if ui.button("apply to map").clicked() {
                vis.iter_mut().for_each(|mut y| {
                    *y = Visibility::Hidden;
                });
                for i in &satcfg.visible {
                    if let Ok(mut s) = vis.get_mut(*i) {
                        *s = Visibility::Visible;
                    }
                }
            }
            if ui.button("update TLE").clicked() {
                let d = query.timer.duration();
                query.timer.set_elapsed(d);
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                if ui.button("export").clicked() {
                    let (tx, rx) = oneshot::channel();
                    satcfg.rx = Some(rx);
                    let _ = rt.0.spawn(async move {
                        let file = AsyncFileDialog::new()
                            .add_filter("csv", &["csv"])
                            .set_directory(env::current_dir().unwrap().as_path())
                            .save_file()
                            .await;
                        println!("send");
                        let _ = tx.send(file);
                    });
          
                }
                if let Some(mut rx) = satcfg.rx.take() {
                    match rx.try_recv() {
                        Ok(f) => {
                            if let Some(filename) = f {
                                println!("{}", filename.file_name());
                                let ts = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs_f64();
                                use std::io::Write;
                                let mut f = std::fs::File::create(filename.path())
                                    .expect("create failed");

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
            create_table(ui, satcfg.table_data.iter());
        });
    uidata.0["Satellite Data"] = opened.into();
}

fn config_ui(
    egui_context: &mut EguiContexts,
    satcfg: &mut ResMut<SatConfigs>,
    gscfg: &mut ResMut<GSConfigs>,
    cccfg: &mut ResMut<ClearColor>,
    opened: &mut bool,
) {
    egui::Window::new("Configs")
        .open(opened)
        .show(egui_context.ctx_mut(), |ui| {
            let a = satcfg.sat_color.clone();
            let mut srgba = unsafe {
                let ptr = (&mut a.to_linear().as_u32() as *mut u32) as *mut u8;

                let srgba = egui::Color32::from_rgba_premultiplied(
                    *ptr.offset(0),
                    *ptr.offset(1),
                    *ptr.offset(2),
                    *ptr.offset(3),
                );
                srgba
            };
            ui.label("Satellite Color:");
            if ui.color_edit_button_srgba(&mut srgba).changed() {
                let (red, green, blue, alpha) = srgba.to_tuple();

                satcfg.sat_color = Color::srgba_u8(red, green, blue, alpha);
            }
            let a = gscfg.color.clone();
            let mut srgba = unsafe {
                let ptr = (&mut a.to_linear().as_u32() as *mut u32) as *mut u8;

                let srgba = egui::Color32::from_rgba_premultiplied(
                    *ptr.offset(0),
                    *ptr.offset(1),
                    *ptr.offset(2),
                    *ptr.offset(3),
                );
                srgba
            };
            ui.label("Ground Station Color:");
            if ui.color_edit_button_srgba(&mut srgba).changed() {
                let (red, green, blue, alpha) = srgba.to_tuple();
      
                gscfg.color = Color::srgba_u8(red, green, blue, alpha);
            }
            ui.label("Clear Color:");
            let a = cccfg.0.clone();
            let mut srgba = unsafe {
                let ptr = (&mut a.to_linear().as_u32() as *mut u32) as *mut u8;

                let srgba = egui::Color32::from_rgba_premultiplied(
                    *ptr.offset(0),
                    *ptr.offset(1),
                    *ptr.offset(2),
                    *ptr.offset(3),
                );
                srgba
            };
            if ui.color_edit_button_srgba(&mut srgba).changed() {
                let (red, green, blue, alpha) = srgba.to_tuple();

                cccfg.0 =  Color::srgba_u8(red, green, blue, alpha);
            }
        });
}

fn create_table<'a, T: ExactSizeIterator + Iterator<Item = &'a [String; 7]>>(
    ui: &mut egui::Ui,
    iter: T,
) {
    let v: Vec<_> = iter.collect();
    let _tb = egui_extras::TableBuilder::new(ui)
        .columns(egui_extras::Column::remainder().resizable(true), 7)
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
            header.col(|ui| {
                ui.heading("inclination");
            });
        })
        .body(|body| {
            body.rows(30.0, v.len(), |mut row| {
                let row_index = row.index();
                let a = &v[row_index];
                for i in *a {
                    row.col(|ui| {
                        ui.text_edit_multiline(&mut i.as_str());
                    });
                }
            });
        });
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]

struct EguiUISet;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
struct InputSet;
fn main() {
    let mut app = App::new();

    app.insert_resource(ClearColor(Color::srgb_u8(0, 7, 13)));
    app.insert_resource(SatConfigs {
        sat_color: Color::srgb_u8(0, 255, 202),
        ..Default::default()
    });
    // ;

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: String::from("RustSat-Satellite Tracking"),
                    ..Default::default()
                }),
                ..default()
            })
            .set(ImagePlugin::default_nearest())
            .build()
            .add_before::<bevy::asset::AssetPlugin, _>(EmbeddedAssetPlugin::default()),
    )
    .add_plugins((
        bevy_svg::prelude::SvgPlugin,
        EguiPlugin,
        // SatRenderPlugin,
        // ShapePlugin,
        // DatalinkPlugin,
    ))
    .add_systems(Startup,setup);

    app.add_plugins(SGP4Plugin);
    #[cfg(feature = "zmq_comm")]
    {
        let mut zmq = zmq_comm::ZMQContext::default();
        zmq.tx_address = "tcp://127.0.0.1:5551".into();
        zmq.rx_address = "tcp://127.0.0.1:5552".into();
        app.insert_resource(zmq);
 
    }
    app.insert_resource(UIData::default());
    app.add_systems(PreUpdate,retro_cam_input_handle.in_set(InputSet));

    app.insert_resource(GSConfigs {
        color: bevy::prelude::Color::Srgba(YELLOW),
        visible: Default::default(),
    });
    app.add_plugins((zmq_comm::ZMQPlugin, GSPlugin));
    //app.add_system_to_stage(CoreStage::PreUpdate, resize_map);
    app.add_systems(PreUpdate, get_cursor_coord);
    app.add_systems(Update,check_vis);
    app.add_systems(Update,show_data.in_set(EguiUISet));
    app.configure_sets(
        Update,
        EguiUISet
            .after(EguiSet::InitContexts)
    );
    // app.add_systems(test);
    app.run();
}

// fn resize_map(
//     mut svg: Query<(&Handle<Svg>, &mut Transform)>,
//     svgs: Res<Assets<Svg>>,
//     mut events: EventReader<WindowResized>,
// ) {
//     for i in events.iter() {
//         svg.iter_mut().for_each(|(s, mut trans)| {
//             let _siz = svgs.get(s).unwrap().size;

//             trans.scale.x = i.width / 1024.0;
//             trans.scale.y = i.height / 1024.0;
//         });
//     }
// }
// fn resize_map2(mut spr: Query<(&Sprite, &mut Transform)>, mut events: EventReader<WindowResized>) {
//     for i in events.iter() {
//         spr.iter_mut().for_each(|(_, mut trans)| {
//             trans.scale[0] = i.width / 1024.0;
//             trans.scale[1] = i.height / 1024.0;
//         });
//     }
// }

// fn cam_input_handle(
//     scroll_evr: EventReader<MouseWheel>,
//     mut ev_motion: EventReader<MouseMotion>,
//     input_mouse: Res<Input<MouseButton>>,

//     mut q: Query<(&mut OrthographicProjection, &mut Transform), With<Camera2d>>,
// ) {
//     let mut acc = 0;
//     scroll_handler(scroll_evr, &mut acc);

//     q.iter_mut().for_each(|(mut x, mut trans)| {
//         let mut zoom = x.scale.ln();
//         zoom += 0.1 * acc as f32;
//         x.scale = zoom.exp();
//         x.scale = x.scale.clamp(0.0, 1.2);

//         if input_mouse.pressed(MouseButton::Middle) {
//             for ev in ev_motion.iter() {
//                 trans.translation = trans.translation - Vec3::new(ev.delta.x, -ev.delta.y, 0.0);
//             }
//             // if trans.translation.x < 0.0 {
//             //     trans.translation.x = 0.0
//             // }
//             // if trans.translation.y < 0.0 {
//             //     trans.translation.y = 0.0
//             // }
//         }
//     });
// }

fn retro_cam_input_handle(
    scroll_evr: EventReader<MouseWheel>,
    mut ev_motion: EventReader<MouseMotion>,
    input_mouse: Res<ButtonInput<MouseButton>>,

    mut q: Query<(&mut OrthographicProjection, &mut Transform), With<Camera2d>>,
) {
    let mut acc = 0;
    scroll_handler(scroll_evr, &mut acc);

    q.iter_mut().for_each(|(mut x, mut trans)| {
        let mut zoom = x.scale.ln();

        zoom += 0.1 * acc as f32;
        //    x.scale = zoom.exp();
        x.scale = zoom.exp();

        if input_mouse.pressed(MouseButton::Middle) {
            for ev in ev_motion.read().into_iter() {
                trans.translation = trans.translation - Vec3::new(ev.delta.x, -ev.delta.y, 0.0);
            }
            // if trans.translation.x < 0.0 {
            //     trans.translation.x = 0.0
            // }
            // if trans.translation.y < 0.0 {
            //     trans.translation.y = 0.0
            // }
        }

        for _ev in ev_motion.read().into_iter(){}
    });
}

fn check_vis(
    q: Query<
        (&OrthographicProjection, &GlobalTransform),
        (With<Camera2d>, Changed<OrthographicProjection>),
    >,
    mut q2: Query<
        (&mut Visibility, &GlobalTransform),
        (With<Mesh2dHandle>, Without<NoFrustumCulling>),
    >,
) {
    q.iter().for_each(|(x, t2)| {
        let dis = x.scale / 2.0;
        let center = Vec2::new(t2.translation().x, t2.translation().y);
        let lb = Vec2::new(center.x - dis, center.y - dis);
        let rb = Vec2::new(center.x + dis, center.y + dis);
        q2.iter_mut().for_each(|(mut vis, transform)| {
            let s = transform.translation();
            *vis = if s.x > lb.x && s.y > lb.y && s.x < rb.x && s.y < rb.y {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        });
    });
}

fn get_cursor_coord(
    mut commands: Commands,
    cc: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    wnds: Query<(&Window,&PrimaryWindow)>,
) {
    let (camera, camera_transform) = cc.single();
    // for wnd in wnds.iter()
    let wnd = wnds.single().0;
    {
        if let Some(screen_pos) = wnd.cursor_position() {
            let window_size = Vec2::new(wnd.width() as f32, wnd.height() as f32);

            // convert screen position [0..resolution] to ndc [-1..1] (gpu coordinates)
            let ndc = (screen_pos / window_size) * 2.0 - Vec2::ONE;
            let ndc_to_world =
                camera_transform.compute_matrix() * camera.clip_from_view().inverse();
            let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));
            // reduce it to a 2D value
            let world_pos: Vec2 = world_pos.truncate();
            commands.insert_resource(CursorPosition(world_pos));
        }
    }
}

fn scroll_handler(mut scroll_evr: EventReader<MouseWheel>, acc: &mut i32) {
    for ev in scroll_evr.read() {
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

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let svg = asset_server.load("webworld2.svg");

    let e1 = commands
        .spawn(GroundStationBundle {
            id: GroundStationID(0),
            pos: LatLonAlt((51.00, -114.029, 0.0)),
        })
        .insert(Name::new("Calgary\nStation 1"))
        .id();

    let e2 = commands
        .spawn(GroundStationBundle {
            id: GroundStationID(1),
            pos: LatLonAlt((44.21895, -80.11, 0.0)),
        })
        .insert(Name::new("Toronto\nStation 2"))
        .id();
    let edge = (e1, e2);
    commands.spawn(GSDataLink(edge)).insert(Name::new("卡多线"));

    let s = asset_server.load_folder("fonts");
  
   

    commands.insert_resource(CursorPosition(Vec2 { x: 0.0, y: 0.0 }));
    // let mut camera = Camera2dBundle::default();

    let mut camera = Camera2dBundle::new_with_far(0.5);

    camera.transform.translation.x = 512.0;
    camera.transform.translation.y = 512.0;

    commands.spawn(camera);

    commands
        .spawn(Svg2dBundle {
            svg,
            origin: Origin::TopLeft,
            transform: Transform::from_xyz(-2.0, 1019.0, 0.0),
            ..Default::default()
        })
        .insert(NoFrustumCulling);
}
