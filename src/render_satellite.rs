use std::f64::consts::PI;

use bevy::{
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
    window::WindowResized,
};

use crate::{celestrak::SatID, SatConfigs};

use super::celestrak::LatLonAlt;
#[derive(Default, Component)]
pub struct WorldCoord(pub Vec2);
#[derive(Default, Resource, Clone)]
pub struct SatelliteMesh(pub Mesh2dHandle, pub Handle<ColorMaterial>);

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
#[allow(dead_code)]
fn wgs84_scaler_define(mut proj: ResMut<GoogleProjector>, mut events: EventReader<WindowResized>) {
    for i in events.read() {
        //proj.zoom = (i.height as i32)/proj.tilesize;

        //let l = (2 as i32).pow(proj.zoom as u32);
        //let f1 = (l  * 256) as f32;
        proj.scaler[1] = i.height / 180.0;
        proj.scaler[0] = i.width / 360.0;
    }
}
fn google_scaler_define(mut proj: ResMut<GoogleProjector>, mut events: EventReader<WindowResized>) {
    for _i in events.read() {
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
    let factor = 0.2;
    if !cam.is_empty() {
        let a = cam.single();

        let not_visible = a.scale > factor;
        q.iter_mut().for_each(|mut f| {
            *f = if not_visible {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
        });
    }
    if !cam2.is_empty() {
        let a = cam2.single();
        let not_visible = a.scale > factor;
        q.iter_mut().for_each(|mut f| {
            *f = if not_visible {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
        });
    }
}

fn google_world_coord(
    mut commands: Commands,
    mut q: Query<(Entity, &LatLonAlt, &mut WorldCoord), Changed<LatLonAlt>>,
    q2: Query<(Entity, &LatLonAlt), Without<WorldCoord>>,
    proj: Res<GoogleProjector>,
) {
    let l = (2 as i32).pow(proj.zoom as u32);
    let _f1 = (l * 256) as f32;
    q.iter_mut().for_each(|(_e, lla, mut w)| {
        //let (xt, yt) = usagi::web_mercator::angle_to_tile(lla.0 .1, lla.0 .0, proj.zoom as u8);

        let coord = add_world_coord(lla, &proj);
        // let coord = usagi::web_mercator::angle_to_pixel(lla.0 .1, lla.0 .0, proj.zoom as u8);
        // let coord = Vec2::new((coord.0 as f32), _f1 - (coord.1 as f32));
        w.0 = coord;
    });

    q2.iter().for_each(|(e, lla)| {
        let xy = add_world_coord(lla, &proj);
        // let xy = usagi::web_mercator::angle_to_pixel(lla.0 .1, lla.0 .0, proj.zoom as u8);
        // let xy = Vec2::new(xy.0 as f32, _f1 - xy.1 as f32);
        commands.entity(e).insert(WorldCoord(xy));
    });
}
#[allow(dead_code)]
fn wgs84_world_coord(
    mut commands: Commands,
    mut q: Query<(Entity, &LatLonAlt, &mut WorldCoord), Changed<LatLonAlt>>,
    q2: Query<(Entity, &LatLonAlt), Added<LatLonAlt>>,
    proj: Res<GoogleProjector>,
) {
    q.iter_mut().for_each(|(_e, lla, mut w)| {
        w.0 = Vec2::from_array([
            proj.scaler[0] * (180.0 + lla.0 .1) as f32,
            proj.scaler[1] * (90.0 + lla.0 .0) as f32,
        ]);
    });

    q2.iter().for_each(|(e, lla)| {
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
    q.iter_mut().for_each(|(mut transform, coord)| {
        transform.translation.x = coord.0.x;
        transform.translation.y = coord.0.y;
    });
}
fn color_update(
    color: Res<SatConfigs>,
    sat_mesh: Option<Res<SatelliteMesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if color.is_changed() && sat_mesh.is_some() {
        let a = materials.get_mut(&sat_mesh.unwrap().1).unwrap();
        a.color = color.sat_color;
    }
}
fn update_labels(
    q: Query<(&LatLonAlt, &Children, &InheritedVisibility), Changed<LatLonAlt>>,
    mut cq: Query<(&mut Text, &SatLabel)>,
) {
    // let mut ss = 0;
    q.iter()
        .filter_map(|(lla, children, &vis)| {
            if !vis.get() {
                return None;
            }
            return Some((lla, children));
        })
        .for_each(|(lla, children)| {
            // ss += 1;
            children.iter().for_each(|child| {
                if let Ok((mut text, label)) = cq.get_mut(*child) {
                    match label {
                        SatLabel::Name => {}
                        SatLabel::Coord => {
                            text.sections[0].value = format!("{:.2}°,{:.2}°", lla.0 .1, lla.0 .0)
                        }
                        SatLabel::Alt => text.sections[0].value = format!("{:.2} km", lla.0 .2),
                    }
                }
            })
        });
}
fn shape_satellite(
    mut commands: Commands,
    color: Res<SatConfigs>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    sat_mesh: Option<Res<SatelliteMesh>>,
    q: Query<(Entity, &WorldCoord, &Name), (Added<WorldCoord>, With<SatID>)>,
) {
    use bevy::prelude::Circle;

    if q.is_empty() {
        return;
    }

    let color = color.sat_color;
    let (mesh_handle, color) = match sat_mesh {
        Some(mesh) => (mesh.0.clone(), mesh.1.clone()),
        None => {
            let mesh = (
                Mesh2dHandle(meshes.add(Circle::default())),
                materials.add(color),
            );
            let s = SatelliteMesh(mesh.0.clone(), mesh.1.clone());
            commands.insert_resource(s);
            mesh
        }
    };

    let text_style = TextStyle {
        font_size: 30.0,
        color: Color::WHITE,
        ..default()
    };

    let bun = MaterialMesh2dBundle {
        mesh: mesh_handle,
        material: color,
        transform: Transform::from_xyz(
            // Distribute shapes from -X_EXTENT/2 to +X_EXTENT/2.
            0.0, 0.0, 0.0,
        ),
        ..default()
    };
    q.iter().for_each(|(e, lla, n)| {
        let xy = lla.0;
        let transform = Transform::from_xyz(xy.x, xy.y, 1.0);
        // let shape = shapes::Circle {
        //     radius: 2.0 / 3.14,
        //     center: Vec2::ZERO,
        // };
        // let shape = ShapeBundle {
        //     path: GeometryBuilder::build_as(&shape),
        //     spatial:SpatialBundle::from_transform(transform),
        //     ..Default::default()
        // };
        let bun2 = bun.clone();

        commands.entity(e).insert(bun2).insert(transform);

        commands.entity(e).with_children(|p| {
            for (i, label) in [
                (n.as_str(), SatLabel::Name),
                ("", SatLabel::Coord),
                ("", SatLabel::Alt),
            ]
            .into_iter()
            .enumerate()
            {
                p.spawn(Text2dBundle {
                    text: Text::from_section(label.0, text_style.clone()),
                    transform: Transform::from_xyz(0.0, -2.0 - i as f32, 1.1)
                        .with_scale([0.04, 0.04, 0.0].into()),
                    ..default()
                })
                .insert(label.1);
            }
        });
    });
}
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
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
        //todo: app.configure_sets(SatRenderStage::SatRenderUpdate.after(CoreSet::PostUpdate).before(Last));
        app.add_systems(
            PostUpdate,
            (
                shape_satellite,
                google_scaler_define,
                color_update,
                update_labels,
            )
                .in_set(SatRenderStage::SatRenderUpdate),
        );

        app.add_systems(PostUpdate, move_satellite);
        app.add_systems(PreUpdate, google_world_coord);

        app.add_systems(PreUpdate, show_label);
    }
}
