use bevy::{color::palettes::css::GREEN, ecs::schedule::ScheduleLabel, prelude::*, render::view::NoFrustumCulling, time::common_conditions::{on_real_timer, on_timer}};
use bevy_prototype_lyon::{prelude::*};
use std::time::Duration;


use crate::{
    celestrak::{LatLonAlt, SatID, TEMEPos},
    groundstation::{GroundStationID, NearestSat},
    render_satellite::{SatRenderStage, WorldCoord},
    util::distance,
};

#[derive(Component)]
pub struct GSDataLink(pub (Entity, Entity));

#[derive(Component)]
pub struct DataLink(pub Vec<DataEdge>);
#[derive(Component)]
pub struct DataLinkLatency(pub f64);

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct InDataLink(pub Entity);

pub struct DataEdge(pub (Entity, Entity));

#[derive(Component, Default)]
pub struct DataLinkStats {
    pub latencies: Vec<f32>,
    pub distance: Vec<f32>,
}
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
    q.iter().for_each(|(entity, v)| {
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
This function is used to establish data links
the realisitic datalink should be established
via LEO satellite network.

*/
pub fn rebuild_gslinks(
    mut cmd: Commands,
    mut q: Query<(&GSDataLink, &mut DataLink)>,
    q2: Query<(&GroundStationID, &NearestSat)>,
) {
    q.iter_mut().for_each(|(v, mut link)| {
        let (a, b) = v.0;
        let res = q2.get(a);
        let res2 = q2.get(b);
        if res.is_err() || res2.is_err() {
            return;
        }
        let res = res.unwrap();
        let res2 = res2.unwrap();
        for i in &link.0 {
            cmd.entity(i.0 .0).remove::<InDataLink>();
            cmd.entity(i.0 .1).remove::<InDataLink>();
        }
        let mut dlink: Vec<_> = Vec::new();
        dlink.push(DataEdge((a, res.1.eid)));
        dlink.push(DataEdge((res.1.eid, res2.1.eid)));

        dlink.push(DataEdge((res2.1.eid, b)));
        *link = DataLink(dlink);
    });
}

/**
When the datalink is established, we should mark the sate or ground
station entity with a flag.
*/
pub fn init_links(mut cmd: Commands, q: Query<(Entity, &DataLink), Changed<DataLink>>) {
    q.iter().for_each(|(entity, v)| {
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
    q.iter().for_each(|(entity, v)| {
        let mut data = DataLinkStats::default();
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
                let gs_llt = q3.get(a).unwrap().1;
                let llt = q3.get(b).unwrap().1;
                dis = distance::ground_space_distance(
                    (gs_llt.0 .0, gs_llt.0 .1),
                    (llt.0 .0, llt.0 .1, 1000.0 * llt.0 .2),
                );
            }

            if is_sat && b_is_sat {
                let gs_llt = q2.get(a).unwrap().2;
                let llt = q2.get(b).unwrap().2;
                let v1 = nalgebra::Vector3::from(gs_llt.0);
                let v2 = nalgebra::Vector3::from(llt.0);
                dis = 1000.0 * v1.metric_distance(&v2);
            }

            if is_sat && b_is_ground {
                let gs_llt = q3.get(a).unwrap().1;
                let llt = q3.get(b).unwrap().1;
                dis = distance::ground_space_distance(
                    (gs_llt.0 .0, gs_llt.0 .1),
                    (llt.0 .0, llt.0 .1, 1000.0 * llt.0 .2),
                );
            }
            sum += dis;
            data.distance.push(dis as f32);
            const LIGHT_SPEED: f64 = 299792458.0;
            data.latencies.push((dis / LIGHT_SPEED) as f32);
        }
        if sum > 0.0 {
            cmd.entity(entity).insert(data);
            cmd.entity(entity).insert(DataLinkLatency(sum));
        }
    });
}

/**
Add a shape to this datalink.
*/
pub fn init_data_link(
    mut commands: Commands,
    q: Query<(Entity, &DataLink), Without<Path>>,
    points: Query<&WorldCoord, With<InDataLink>>,
) {
    q.iter().for_each(|(entity, v)| {
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
        let mut t = Transform::default();
        t.translation.z = 1.0f32;
        let stroke = Stroke::new(GREEN, 0.1);
        let shape = ShapeBundle {
            path:line ,
            spatial: SpatialBundle::from_transform(t),
            ..Default::default()
        };
        commands
            .entity(entity)
            .insert((NoFrustumCulling,shape,stroke))
            ;
    });
}

/**
update data link path in real time because satellite is moving.
*/
pub fn update_data_link(
    mut q: Query<(Entity, &DataLink, &mut Path)>,
    points: Query<&WorldCoord, With<InDataLink>>,
) {
    q.iter_mut().for_each(|(_entity, v, mut path)| {
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
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum LinkRenderStage {
    RenderUpdate,
}
pub struct DatalinkPlugin;
impl Plugin for DatalinkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (init_gslinks, init_links, init_data_link, update_data_link)
                .in_set(LinkRenderStage::RenderUpdate)
                .chain(),
        );

        app.configure_sets(Update,LinkRenderStage::RenderUpdate.after(SatRenderStage::SatRenderUpdate));
        app.add_systems(Update,rebuild_gslinks.run_if(on_real_timer(Duration::from_secs_f32(10.0))));
        // .with_system(
        //     rebuild_gslinks
        //         .with_run_criteria(FixedTimestep::step(10.0))
        //         .after(update_data_link),
        // )

        app.add_systems(Update,compute_latency.in_set(LinkRenderStage::RenderUpdate));
    }
}
