use bevy::{prelude::*, render::view::NoFrustumCulling};
use bevy_prototype_lyon::{prelude::*, render::Shape};

use crate::{
    celestrak::{LatLonAlt, SatID, TEMEPos},
    groundstation::{GroundStationID, NearestSat},
    render_satellite::WorldCoord,
    util::distance,
};

#[derive(Component)]
pub struct GSDataLink(pub (Entity, Entity));

#[derive(Component)]
pub struct DataLink(pub Vec<DataEdge>);
#[derive(Component)]
pub struct DataLinkLatency(pub f64);
#[derive(Component)]
pub struct InDataLink(pub Entity);

pub struct DataEdge(pub (Entity, Entity));

/**
This function is used to establish data links
the realisitic datalink should be established
via LEO satellite network.

*/
pub fn init_gslinks(
    mut cmd: Commands,
    q: Query<(Entity, &GSDataLink), Without<DataLink>>,
    q2: Query<(&GroundStationID, &NearestSat)>,
) {
    q.for_each(|(entity, v)| {
        let (a, b) = v.0;
        let res = q2.get(a);
        let res2 = q2.get(b);
        if res.is_err() || res2.is_err() {
            return;
        }
        let res = res.unwrap();
        let res2 = res2.unwrap();

        let mut dlink: Vec<_> = Vec::new();
        dlink.push(DataEdge((a, res.1.eid)));
        dlink.push(DataEdge((res.1.eid, res2.1.eid)));

        dlink.push(DataEdge((res2.1.eid, b)));

        cmd.entity(entity).insert(DataLink(dlink));
    });
}

/**
When the datalink is established, we should mark the sate or ground
station entity with a flag.
*/
pub fn init_links(mut cmd: Commands, q: Query<(Entity, &DataLink), Added<DataLink>>) {
    q.for_each(|(entity, v)| {
        let v = &v.0;
        for i in v {
            let (a, b) = i.0;
            cmd.entity(a).insert(InDataLink(entity));
            cmd.entity(b).insert(InDataLink(entity));
        }
    });
}

/**
When the datalink is established, we can compute the latency of this link.
*/
pub fn compute_latency(
    mut cmd: Commands,
    q: Query<(Entity, &DataLink)>,
    q2: Query<(&SatID, &LatLonAlt, &TEMEPos), With<InDataLink>>,
    q3: Query<(&GroundStationID, &LatLonAlt), With<InDataLink>>,
) {
    q.for_each(|(entity, v)| {
        let v = &v.0;
        let mut sum = 0.0;
        for i in v {
            let (a, b) = i.0;
            let is_sat = q2.contains(a);
            let is_ground = q3.contains(a);
            let b_is_sat = q2.contains(b);
            let b_is_ground = q3.contains(b);
            let mut dis = 0.0;
            if is_ground && b_is_sat {
                let gs_llt = q3.get_component::<LatLonAlt>(a).unwrap();
                let llt = q2.get_component::<LatLonAlt>(b).unwrap();
                dis = distance::ground_space_distance(
                    (gs_llt.0 .0, gs_llt.0 .1),
                    (llt.0 .0, llt.0 .1, 1000.0 * llt.0 .2),
                );
            }

            if is_sat && b_is_sat {
                let gs_llt = q2.get_component::<TEMEPos>(a).unwrap();
                let llt = q2.get_component::<TEMEPos>(b).unwrap();
                let v1 = nalgebra::Vector3::from(gs_llt.0);
                let v2 = nalgebra::Vector3::from(llt.0);
                dis = 1000.0 * v1.metric_distance(&v2);
            }

            if is_sat && b_is_ground {
                let gs_llt = q3.get_component::<LatLonAlt>(b).unwrap();
                let llt = q2.get_component::<LatLonAlt>(a).unwrap();
                dis = distance::ground_space_distance(
                    (gs_llt.0 .0, gs_llt.0 .1),
                    (llt.0 .0, llt.0 .1, 1000.0 * llt.0 .2),
                );
            }
            sum += (dis / 3e8);
        }

        cmd.entity(entity).insert(DataLinkLatency(sum));
    });
}

/**
Add a shape to this datalink.
*/
pub fn init_data_link(
    mut commands: Commands,
    q: Query<(Entity, &DataLink), Without<Shape>>,
    points: Query<&WorldCoord, (With<InDataLink>)>,
) {
    q.for_each(|(entity, v)| {
        let v = &v.0;
        let mut path_builder = PathBuilder::new();

        for i in v {
            let (a, b) = i.0;
            let spos = points.get(a);
            let pos = points.get(b);
            if spos.is_err() || pos.is_err() {
                return;
            }
            let spos = spos.unwrap();
            let pos = pos.unwrap();
            path_builder.move_to(spos.0);
            path_builder.line_to(pos.0);
        }
        let line = path_builder.build();

        commands.entity(entity).insert(GeometryBuilder::build_as(
            &line,
            DrawMode::Stroke(StrokeMode::new(Color::GREEN, 0.1)),
            Transform::default(),
        )).insert(NoFrustumCulling);
    });
}


/**
update data link path in real time because satellite is moving.
*/
pub fn update_data_link(
    mut q: Query<(Entity, &DataLink,&mut Path)>,
    points: Query<&WorldCoord, (With<InDataLink>)>,
) {
    q.for_each_mut(|(entity, v,mut path)| {
        let v = &v.0;
        let mut path_builder = PathBuilder::new();

        for i in v {
            let (a, b) = i.0;
            let spos = points.get(a);
            let pos = points.get(b);
            if spos.is_err() || pos.is_err() {
                return;
            }

            let spos = spos.unwrap();
            let pos = pos.unwrap();
            path_builder.move_to(spos.0);
            path_builder.line_to(pos.0);
        }
        let line = path_builder.build();

        *path = line;
       
        // commands.entity(entity).insert(GeometryBuilder::build_as(
        //     &line,
        //     DrawMode::Stroke(StrokeMode::new(Color::GREEN, 10.0)),
        //     Transform::default(),
        // ));
    });
}
#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum LinkRenderStage {
    RenderUpdate,
}
pub struct DatalinkPlugin;
impl Plugin for DatalinkPlugin {
    fn build(&self, app: &mut App) {
        app.add_stage_after(
            CoreStage::PostUpdate,
            LinkRenderStage::RenderUpdate,
            SystemStage::parallel()
                .with_system(init_gslinks)
                .with_system(init_links)
                .with_system(init_data_link)
                .with_system(update_data_link),
        );
        app.add_system(compute_latency);
    }
}
