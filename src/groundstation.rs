use crate::render_satellite::{SatRenderStage, WorldCoord};
use crate::util::*;

use crate::celestrak::{LatLonAlt, SatID};
use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
#[derive(Component, Default)]
pub struct GroundStationID(pub u64);


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
// pub fn distance_init(
//     mut cmd: Commands,
//     q: Query<Entity, (With<GroundStationID>, Without<NearestSat>)>,
// ) {
//     q.for_each(|e| {
//         cmd.entity(e).insert(NearestSat {
//             eid: e,
//             distance: 0.0,
//         });
//     });
// }
pub fn distance_update(
    mut commands: Commands,
    mut q: Query<(Entity,&GroundStationID, &LatLonAlt)>,
    sats: Query<(Entity, &SatID, &LatLonAlt)>,
) {
    q.iter_mut().for_each(|(entity,_id, gs_llt)| {
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
        commands.entity(entity).insert( res.unwrap());

    });
}
pub fn print_gs(q: Query<(&GroundStationID, &Transform)>) {
    q.iter().for_each(|(_id, trans)| {
        info!("{}", trans.translation);
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
    q.iter().for_each(|(e, lla, n)| {
        info!("{}", lla.0);
        let xy = lla.0;
        let transform = Transform::from_xyz(xy.x, xy.y, 1.0);
        let shape = shapes::Circle {
            radius: 2.0 / 3.14,
            center: Vec2::ZERO,
        };
        let shape = ShapeBundle {
            path:GeometryBuilder::build_as(
                &shape,
            ),
            spatial:SpatialBundle::from_transform(transform),
            ..Default::default()
        };

        commands.entity(e).insert((shape,Fill::color(color.color)));
        commands.entity(e).clear_children();
        commands.entity(e).with_children(|parent| {
            parent.spawn(Text2dBundle {
                text: Text::from_section(n.as_str(), text_style.clone()),
                transform: Transform::from_xyz(0.0, -2.0, 1.1).with_scale([0.04, 0.04, 0.0].into()),
                ..default()
            });
        });
    });
}

fn color_update(

    color: Res<GSConfigs>,
    mut q: Query<& mut Fill, With<GroundStationID>>,

) {
    if color.is_changed() {
        q.iter_mut().for_each(| mut c| {
            *c = Fill::color(color.color);
        });
    }
}

pub struct GSPlugin;

impl Plugin for GSPlugin {
    fn build(&self, app: &mut App) {
        //app.add_system_to_stage(CoreStage::PreUpdate, distance_init);
        app.add_systems(PostUpdate, distance_update);
        //app.add_systems(print_gs);
        app.add_systems( PreUpdate, shape_ground_station);
        app.add_systems( Update,color_update.in_set(SatRenderStage::SatRenderUpdate));

    }
}
