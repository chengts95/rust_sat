use std::f64::consts::PI;

use bevy::{prelude::*, window::WindowResized};
use bevy_prototype_lyon::prelude::*;

use crate::{celestrak::SatID, SatConfigs};

use super::celestrak::LatLonAlt;
#[derive(Default, Component)]
struct WorldCoord(Vec2);

#[derive(Resource)]
pub struct GoogleProjector {
    pub zoom: i32,
    pub scaler: Vec2,
    pub initial_resolution: f64,
    pub origin_shift: f64,
    pub tilesize: i32,
}
impl GoogleProjector {
    pub fn latlon_to_meters(&self, lat: f64, lon: f64) -> (f64, f64) {
        let mx = lon * self.origin_shift / 180.0;
        let mut my = ((90.0 + lat) * PI / 360.0).tan().ln() / (PI / 180.0);

        my = my * self.origin_shift / 180.0;
        (mx, my)
    }
    pub fn meters_to_pixels(&self, mx: f64, my: f64) -> (f64, f64) {
        let res = self.resolution();
        let px = (mx + self.origin_shift) / res;
        let py = (my + self.origin_shift) / res;
        (px, py)
    }

    pub fn resolution(&self) -> f64 {
        self.initial_resolution / ((2 as i32).pow(self.zoom as u32) as f64)
    }
}

impl Default for GoogleProjector {
    fn default() -> Self {
        let tilesize = 256;
        Self {
            zoom: Default::default(),
            scaler: Default::default(),
            initial_resolution: 2.0 * PI * 6378137.0 / (tilesize as f64),
            origin_shift: PI * 6378137.0,
            tilesize,
        }
    }
}
#[derive(Component)]
pub enum SatLabel {
    Name,
    Coord,
    Alt,
}
fn wgs84_scaler_define(mut proj: ResMut<GoogleProjector>, mut events: EventReader<WindowResized>) {
    for i in events.iter() {
        //proj.zoom = (i.height as i32)/proj.tilesize;

        //let l = (2 as i32).pow(proj.zoom as u32);
        //let f1 = (l  * 256) as f32;
        proj.scaler[1] = i.height / 180.0;
        proj.scaler[0] = i.width / 360.0;
    }
}
fn google_scaler_define(mut proj: ResMut<GoogleProjector>, mut events: EventReader<WindowResized>) {
    for _i in events.iter() {
        proj.zoom = 2; //(i.height as i32)/proj.tilesize;

        let l = (2 as i32).pow(proj.zoom as u32);
        let _f1 = (l * 256) as f32;
        proj.scaler[1] = 1.0; //i.height/f1;
        proj.scaler[0] = 1.0 //i.width/f1;
    }
}

fn show_label(
    cam: Query<&OrthographicProjection, Changed<OrthographicProjection>>,
    cam2: Query<&OrthographicProjection, Added<OrthographicProjection>>,
    mut q: Query<&mut Visibility, With<SatLabel>>,
) {
    let factor = 256.0;
    if !cam.is_empty() {
        let a = cam.single();

        let not_visible = a.scale > factor;
        q.for_each_mut(|mut f| {
            f.is_visible = !not_visible;
        });
    }
    if !cam2.is_empty() {
        let a = cam2.single();
        let not_visible = a.scale > factor;
        q.for_each_mut(|mut f| {
            f.is_visible = !not_visible;
        });
    }
}
fn google_world_coord(
    mut commands: Commands,
    mut q: Query<(Entity, &LatLonAlt, &mut WorldCoord), Changed<LatLonAlt>>,
    q2: Query<(Entity, &LatLonAlt), Added<LatLonAlt>>,
    proj: Res<GoogleProjector>,
) {
    q.for_each_mut(|(_e, lla, mut w)| {
        let coord = add_world_coord(lla, &proj);
        w.0 = coord;
    });

    q2.for_each(|(e, lla)| {
        let xy = add_world_coord(lla, &proj);
        commands.entity(e).insert(WorldCoord(xy));
    });
}
fn wgs84_world_coord(
    mut commands: Commands,
    mut q: Query<(Entity, &LatLonAlt, &mut WorldCoord), Changed<LatLonAlt>>,
    q2: Query<(Entity, &LatLonAlt), Added<LatLonAlt>>,
    proj: Res<GoogleProjector>,
) {
    q.for_each_mut(|(_e, lla, mut w)| {
        w.0 = Vec2::from_array([
            proj.scaler[0] * (180.0 + lla.0 .1) as f32,
            proj.scaler[1] * (90.0 + lla.0 .0) as f32,
        ]);
    });

    q2.for_each(|(e, lla)| {
        let xy = Vec2::from_array([
            proj.scaler[0] * (180.0 + lla.0 .1) as f32,
            proj.scaler[1] * (90.0 + lla.0 .0) as f32,
        ]);
        commands.entity(e).insert(WorldCoord(xy));
    });
}
fn add_world_coord(lla: &LatLonAlt, proj: &Res<GoogleProjector>) -> bevy::prelude::Vec2 {
    let (lat, lon, _) = lla.0;
    let (mx, my) = proj.latlon_to_meters(lat, lon);
    let coord = proj.meters_to_pixels(mx, my);

    let xy = Vec2::from_slice(&[
        proj.scaler.x * coord.0 as f32,
        proj.scaler.y * coord.1 as f32,
    ]);
    xy
}

fn move_satellite(mut q: Query<(&mut Transform, &WorldCoord), Changed<WorldCoord>>) {
    q.for_each_mut(|(mut transform, coord)| {
        transform.translation.x = coord.0.x;
        transform.translation.y = coord.0.y;
    });
}
fn color_update(color: Res<SatConfigs>, mut q: Query<(&SatID, &mut DrawMode)>) {
    if color.is_changed() {
        q.for_each_mut(|(_a, mut c)| {
            *c = DrawMode::Fill {
                0: FillMode::color(color.sat_color),
            };
        });
    }
}
fn update_labels(
    q: Query<(&LatLonAlt, &Children), Changed<LatLonAlt>>,
    mut cq: Query<(&mut Text, &SatLabel, &ComputedVisibility)>,
) {
    q.for_each(|(lla, children)| {
        for child in children {
            if let Ok((mut text, label, vis)) = cq.get_mut(*child) {
                if !vis.is_visible() {
                    continue;
                }
                match *label {
                    SatLabel::Name => {}
                    SatLabel::Coord => {
                        text.sections[0].value = format!("{:.2}°,{:.2}°", lla.0 .1, lla.0 .0)
                    }
                    SatLabel::Alt => text.sections[0].value = format!("{:.2} km", lla.0 .2),
                    _ => unreachable!(),
                }
            }
        }
    });
}
fn shape_satellite(
    mut commands: Commands,
    color: Res<SatConfigs>,
    q: Query<(Entity, &WorldCoord, &Name), Added<WorldCoord>>,
    fonts: Query<&Handle<Font>>,
) {
    if q.is_empty() {
        return;
    }
    let font = fonts.single().clone();
    let text_style = TextStyle {
        font,
        font_size: 30.0,
        color: Color::WHITE,
    };
    q.for_each(|(e, lla, n)| {
        let xy = lla.0;
        let trans = Transform::from_xyz(xy.x, xy.y, 1.0);
        let shape = shapes::Circle {
            radius: 1.0 / 3.14,
            center: Vec2::ZERO,
        };
        commands
            .entity(e)
            .insert(GeometryBuilder::build_as(
                &shape,
                DrawMode::Fill {
                    0: FillMode::color(color.sat_color),
                },
                trans,
            ))
            .insert(VisibilityBundle::default());

        commands.entity(e).with_children(|parent| {
            parent
                .spawn_bundle(Text2dBundle {
                    text: Text::from_section(n.as_str(), text_style.clone())
                        .with_alignment(TextAlignment::CENTER),
                    transform: Transform::from_xyz(0.0, -2.0, 1.1)
                        .with_scale([0.04, 0.04, 0.0].into()),
                    ..default()
                })
                .insert(SatLabel::Name);
            parent
                .spawn_bundle(Text2dBundle {
                    text: Text::from_section("", text_style.clone())
                        .with_alignment(TextAlignment::CENTER),
                    transform: Transform::from_xyz(0.0, -3.0, 1.1)
                        .with_scale([0.04, 0.04, 0.0].into()),
                    ..default()
                })
                .insert(SatLabel::Coord);
            parent
                .spawn_bundle(Text2dBundle {
                    text: Text::from_section("", text_style.clone())
                        .with_alignment(TextAlignment::CENTER),
                    transform: Transform::from_xyz(0.0, -4.0, 1.1)
                        .with_scale([0.04, 0.04, 0.0].into()),
                    ..default()
                })
                .insert(SatLabel::Alt);
        });
    });
}
#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum SatRenderStage {
    SatRenderUpdate,
}
#[derive(Default)]
pub struct SatRenderPlugin;

impl Plugin for SatRenderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GoogleProjector {
            zoom: 2,
            scaler: Vec2 { x: 1.0, y: 1.0 },
            ..default()
        });
        app.add_stage_after(
            CoreStage::PostUpdate,
            SatRenderStage::SatRenderUpdate,
            SystemStage::parallel()
                .with_system(shape_satellite)
                .with_system(google_scaler_define)
                .with_system(color_update)
                .with_system(update_labels),
        );
        app.add_system_to_stage(CoreStage::PostUpdate, move_satellite);
        app.add_system_to_stage(CoreStage::PreUpdate, google_world_coord);

        app.add_system_to_stage(CoreStage::PreUpdate, show_label);
        let shape = shapes::Circle {
            radius: 2.0,
            center: Vec2::ZERO,
        };
        app.world.spawn(GeometryBuilder::build_as(
            &shape,
            DrawMode::Fill {
                0: FillMode::color(Color::YELLOW),
            },
            Transform::from_xyz(0.0, 0.0, 0.0),
        ));
    }
}
