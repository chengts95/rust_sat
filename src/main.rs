//"https://satellitemap.space/json"
#![allow(dead_code)]
#![allow(unused_imports)]
use bevy::{
    asset::load_internal_binary_asset,
    color::palettes::css::YELLOW,
    input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel},
    prelude::*,
    render::view::NoFrustumCulling,
    sprite::Mesh2dHandle,
    window::PrimaryWindow,
};

use bevy_egui::{EguiPlugin, EguiSet};
use bevy_embedded_assets::EmbeddedAssetPlugin;
use bevy_prototype_lyon::prelude::ShapePlugin;

use datalink::{DatalinkPlugin, GSDataLink};

use groundstation::{GSConfigs, GSPlugin, GroundStationBundle, GroundStationID};

use sgp4::Orbit;

use bevy_svg::prelude::*;
pub mod celestrak;
mod cfg_ui;
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

use celestrak::*;
use cfg_ui::*;
use render_satellite::*;

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
        SatRenderPlugin,
        ShapePlugin,
        DatalinkPlugin,
    ))
    .add_systems(Startup, setup);

    app.add_plugins(SGP4Plugin);
    #[cfg(feature = "zmq_comm")]
    {
        let mut zmq = zmq_comm::ZMQContext::default();
        zmq.tx_address = "tcp://127.0.0.1:5551".into();
        zmq.rx_address = "tcp://127.0.0.1:5552".into();
        app.insert_resource(zmq);
    }
    app.insert_resource(UIData::default());
    app.add_systems(PreUpdate, retro_cam_input_handle.in_set(InputSet));

    app.insert_resource(GSConfigs {
        color: bevy::prelude::Color::Srgba(YELLOW),
        visible: Default::default(),
    });
    app.add_plugins(GSPlugin);
    //app.add_system_to_stage(CoreStage::PreUpdate, resize_map);
    app.add_systems(PreUpdate, get_cursor_coord);
    //app.add_systems(Update,check_vis);
    app.add_systems(Update, show_data.in_set(EguiUISet));
    app.configure_sets(Update, EguiUISet.after(EguiSet::InitContexts));
    // app.add_systems(test);

    load_internal_binary_asset!(
        app,
        TextStyle::default().font,
        "../assets/fonts/simhei.ttf",
        |bytes: &[u8], _path: String| { Font::try_from_bytes(bytes.to_vec()).unwrap() }
    );
    app.run();
}

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

        for _ev in ev_motion.read().into_iter() {}
    });
}

fn check_vis(
    q2: Query<(&ViewVisibility, &GlobalTransform), (With<Mesh2dHandle>, Without<NoFrustumCulling>)>,
) {
    let mut visible = 0;
    let mut total = 0;
    q2.iter().for_each(|(vis, _transform)| {
        if vis.get() {
            visible += 1;
        }
        total += 1;
    });
    bevy::log::info!("Visible:{} Total:{}", visible, total);
}

fn get_cursor_coord(
    mut commands: Commands,
    cc: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    wnds: Query<(&Window, &PrimaryWindow)>,
) {
    let (camera, camera_transform) = cc.single();
    // for wnd in wnds.iter()
    if let Ok((wnd, _)) = wnds.get_single() {
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
    println!("{:?}", s);
    commands.insert_resource(CursorPosition(Vec2 { x: 0.0, y: 0.0 }));
    // let mut camera = Camera2dBundle::default();

    let mut camera = Camera2dBundle::new_with_far(1000.0);

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
