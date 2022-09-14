use bevy::{prelude::*, window::WindowResized};
use bevy_prototype_lyon::prelude::*;

use crate::{celestrak::SatID, SatConfigs};

use super::celestrak::LatLonAlt;
#[derive(Default, Component)]
struct WorldCoord(Vec2);
pub struct GoogleProjector {
    pub zoom: i32,
    pub scaler: Vec2,
}
#[derive(Component)]
pub struct SatLabel;

fn google_scaler_define(mut proj: ResMut<GoogleProjector>, mut events: EventReader<WindowResized>) {
    for i in events.iter() {
        proj.zoom = (i.height / 256.0) as i32;
        let l = (2 as i32).pow(proj.zoom as u32);
        let f1 = l as f32 * 256.0;
        proj.scaler[1] = i.height / f1;
        proj.scaler[0] = i.width / f1;
    }
}

fn show_label(
    cam: Query<&OrthographicProjection, Changed<OrthographicProjection>>,
    mut q: Query<&mut Visibility, With<SatLabel>>,
) {
    if !cam.is_empty() {
        let a = cam.single();
        let not_visible = a.scale > 0.2;
        q.for_each_mut(|mut f| {
            f.is_visible = !not_visible;
        });
    }
}
fn google_world_coord(
    mut commands: Commands,
    q: Query<(Entity, &LatLonAlt), Changed<LatLonAlt>>,
    q2: Query<(Entity, &LatLonAlt), Added<LatLonAlt>>,
    proj: Res<GoogleProjector>,
) {
    q.for_each(|(e, lla)| {
        add_world_coord(lla, &proj, &mut commands, e);
    });
    q2.for_each(|(e, lla)| {
        add_world_coord(lla, &proj, &mut commands, e);
    });
}

fn add_world_coord(
    lla: &LatLonAlt,
    proj: &Res<GoogleProjector>,
    commands: &mut Commands,
    e: Entity,
) {
    let (lat, lon, _) = lla.0;
    let coord = googleprojection::from_ll_to_pixel(&(lon, lat), proj.zoom as usize);
    if let Some(coord) = coord {
        let xy = Vec2::from_slice(&[
            proj.scaler.x * coord.0 as f32,
            proj.scaler.y * coord.1 as f32,
        ]);
        commands.entity(e).insert(WorldCoord(xy));
    }
}

fn move_satellite(mut q: Query<(&mut Transform, &WorldCoord), Changed<WorldCoord>>) {
    q.for_each_mut(|(mut transform, coord)| {
        transform.translation.x = coord.0.x;
        transform.translation.y = -coord.0.y;
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
        font_size: 60.0,
        color: Color::WHITE,
    };
    q.for_each(|(e, lla, n)| {
        let xy = lla.0;
        let trans = Transform::from_xyz(xy.x, xy.y, 1.0);
        let shape = shapes::Circle {
            radius: 1.0,
            center: Vec2::ZERO,
        };
        commands
            .entity(e)
            .insert_bundle(GeometryBuilder::build_as(
                &shape,
                DrawMode::Fill {
                    0: FillMode::color(color.sat_color),
                },
                trans,
            ))
            .insert_bundle(VisibilityBundle::default());

        commands.entity(e).with_children(|parent| {
            parent
                .spawn_bundle(Text2dBundle {
                    text: Text::from_section(n.as_str(), text_style.clone())
                        .with_alignment(TextAlignment::CENTER),
                    transform: Transform::from_xyz(0.0, 3.0, 1.1)
                        .with_scale([0.02, 0.02, 0.0].into()),
                    ..default()
                })
                .insert(SatLabel);
            parent
                .spawn_bundle(Text2dBundle {
                    text: Text::from_section(n.as_str(), text_style.clone())
                        .with_alignment(TextAlignment::CENTER),
                    transform: Transform::from_xyz(0.0, 3.0, 1.1)
                        .with_scale([0.02, 0.02, 0.0].into()),
                    ..default()
                })
                .insert(SatLabel);
            parent
                .spawn_bundle(Text2dBundle {
                    text: Text::from_section(n.as_str(), text_style.clone())
                        .with_alignment(TextAlignment::CENTER),
                    transform: Transform::from_xyz(0.0, 3.0, 1.1)
                        .with_scale([0.02, 0.02, 0.0].into()),
                    ..default()
                })
                .insert(SatLabel);
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
            zoom: 6,
            scaler: Vec2 { x: 1.0, y: 1.0 },
        });
        app.add_stage_after(
            CoreStage::PostUpdate,
            SatRenderStage::SatRenderUpdate,
            SystemStage::parallel()
                .with_system(shape_satellite)
                .with_system(google_scaler_define)
                .with_system(color_update),
        );
        app.add_system_to_stage(CoreStage::PostUpdate, move_satellite);
        app.add_system_to_stage(CoreStage::PreUpdate, google_world_coord);

        app.add_system_to_stage(CoreStage::PreUpdate, show_label);
        // let shape = shapes::Circle {
        //     radius: 2.0,
        //     center: Vec2::ZERO,
        // };
        // app.world.spawn().insert_bundle(GeometryBuilder::build_as(
        //     &shape,
        //     DrawMode::Fill{
        //         0: FillMode::color(Color::BLACK),
        //     },
        //     Transform::from_xyz(0.0, 0.0,0.0)

        // ));
    }
}
