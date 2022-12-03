use crate::render_satellite::{SatRenderStage, WorldCoord};
use crate::util::*;

use crate::celestrak::{LatLonAlt, SatID};
use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
#[derive(Component, Default)]
pub struct GroundStationID(pub u64);

#[derive(Component)]
pub struct GSDataLink(pub (Entity, Entity));
#[derive(Default, Resource)]
pub struct GSConfigs {
    pub color: Color,
    //table_data: Vec<[String; 6]>,
    pub visible: Vec<Entity>,
}
#[derive(Component)]
pub struct NearestSat {
    pub eid: Entity,
    pub distance: f64,
}

#[derive(Bundle)]
pub struct GroundStationBundle {
    pub id: GroundStationID,
    pub pos: LatLonAlt,
}
pub fn distance_init(
    mut cmd: Commands,
    q: Query<Entity, (With<GroundStationID>, Without<NearestSat>)>,
) {
    q.for_each(|e| {
        cmd.entity(e).insert(NearestSat {
            eid: e,
            distance: 0.0,
        });
    });
}
pub fn distance_update(
    mut q: Query<(&GroundStationID, &LatLonAlt, &mut NearestSat)>,
    sats: Query<(Entity, &SatID, &LatLonAlt)>,
) {
    q.for_each_mut(|(_id, gs_llt, mut neareast)| {
        let res = sats
            .iter()
            .map(|(e, _, llt)| {
                let dis = distance::ground_space_distance((gs_llt.0 .0, gs_llt.0 .1), (llt.0.0,llt.0.1,1000.0*llt.0.2));
                NearestSat {
                    eid: e,
                    distance: dis,
                }
            })
            .min_by(|x, y| x.distance.partial_cmp(&y.distance).unwrap());
        if res.is_none() {
            return;
        }
        *neareast = res.unwrap();
    });
}
pub fn print_gs(q: Query<(&GroundStationID, &LatLonAlt, &NearestSat, &WorldCoord)>) {
    q.for_each(|(_id, _llt, sat, _w)| {
        info!("{}", sat.distance);
    });
}

fn shape_ground_station(
    mut commands: Commands,
    color: Res<GSConfigs>,
    q: Query<(Entity, &WorldCoord, &Name), (With<GroundStationID>, Added<WorldCoord>)>,
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
        info!("{}", lla.0);
        let xy = lla.0;
        let trans = Transform::from_xyz(xy.x, xy.y, 1.0);
        let shape = shapes::Circle {
            radius: 1.0 / 3.14,
            center: Vec2::ZERO,
        };
        commands.entity(e).insert(GeometryBuilder::build_as(
            &shape,
            DrawMode::Fill {
                0: FillMode::color(color.color),
            },
            trans,
        ));

        commands.entity(e).with_children(|parent| {
            parent.spawn(Text2dBundle {
                text: Text::from_section(n.as_str(), text_style.clone())
                    .with_alignment(TextAlignment::CENTER),
                transform: Transform::from_xyz(0.0, -2.0, 1.1).with_scale([0.04, 0.04, 0.0].into()),
                ..default()
            });
        });
    });
}
fn print_cam(cam: Query<(&OrthographicProjection, &GlobalTransform)>) {
    cam.for_each(|(a, _camera_transform)| {
        let _s = a.left;
    });
}
pub struct GSPlugin;

impl Plugin for GSPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_to_stage(CoreStage::PreUpdate, distance_init);
        app.add_system_to_stage(CoreStage::PostUpdate, distance_update);
        app.add_system_to_stage(CoreStage::PostUpdate, print_gs);
        app.add_system_to_stage(SatRenderStage::SatRenderUpdate, shape_ground_station);
        app.add_system_to_stage(SatRenderStage::SatRenderUpdate, print_cam);
    }
}
