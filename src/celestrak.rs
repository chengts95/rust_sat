//7R7C44-NVYFK3-9VFVKJ-4X3J
//"https://api.n2yo.com/rest/v1/satellite/"
//tle/52997&apiKey=

use crate::{QueryConfig, TaskWrapper};
use bevy::prelude::*;

use std::{
    collections::HashMap,
    time::{Duration, UNIX_EPOCH},
};

use chrono::Datelike;
use chrono::{DateTime, TimeZone, Timelike, Utc};
use derive_more::{From, Into};
use serde::{Deserialize, Serialize};
use sgp4::{Constants, Elements};
use tokio::runtime::Runtime;
//https://celestrak.org/NORAD/elements/gp.php?GROUP=STARLINK&FORMAT=TLE
pub(crate) async fn get_sat_data() -> Result<Vec<sgp4::Elements>, reqwest::Error> {
    let resp =
        reqwest::get("https://celestrak.org/NORAD/elements/gp.php?GROUP=STARLINK&FORMAT=JSON")
            .await?
            .json::<Vec<sgp4::Elements>>()
            .await;
    resp
}
#[derive(Component, serde::Serialize, serde::Deserialize)]
pub struct CElements(sgp4::Elements);
#[derive(Default, Component, From, Into)]
pub struct SatName(pub String);


#[derive(Default, Component, From, Into)]
pub struct SatID(pub u64);

#[derive(Default, Component, From, Into)]
pub struct TEMEPos(pub [f64; 3]);
#[derive(Default, Component, From, Into)]
pub struct TEMEVelocity(pub [f64; 3]);

#[derive(Component, From, Into)]
pub struct SGP4Constants(pub Constants<'static>);

#[derive(Component, From, Into)]
pub struct LatLonAlt(pub (f64, f64, f64));

#[derive(Component, From, Into)]
pub struct TLETimeStamp(pub i64);
#[derive(Default, Serialize, Deserialize)]
pub struct SatInfo {
    pub sats: HashMap<u64, sgp4::Elements>,
}
pub fn get_name(data: &Res<SatInfo>, id: &&SatID) -> String {
    data.sats
        .get(&id.0)
        .unwrap()
        .object_name
        .as_ref()
        .unwrap()
        .to_owned()
}

fn update_data(
    mut cmd: Commands,
    rt: Res<Runtime>,
    mut config: ResMut<QueryConfig>,
    time: Res<bevy::time::Time>,
) {
    config.timer.tick(time.delta());

    if config.timer.finished() {
        let task = rt.spawn(async { get_sat_data().await.unwrap() });
        cmd.spawn().insert(TaskWrapper(Some(task)));
        // info!(
        //     "Query the satellite info at {}",
        //     time.seconds_since_startup()
        // );
    }
}
fn receive_task(
    mut cmd: Commands,
    rt: Res<Runtime>,
    mut tasks: Query<(Entity, &mut TaskWrapper<Vec<Elements>>)>,
    mut sat: ResMut<SatInfo>,
) {
    tasks.for_each_mut(|(e, mut t)| {
        if t.0.is_some() {
            if t.0.as_ref().unwrap().is_finished() {
                let s = t.0.take().unwrap();
                let res = rt.block_on(s).unwrap();
                let mut sat_info = SatInfo::default();
                for elements in res {
                    sat_info.sats.insert(elements.norad_id, elements);
                }
                *sat = sat_info;
                //info!("Meassge Received! {}", sat.sats.len());
                //evts.send(QueriedEvent::default());
            }
        }

        if t.0.is_none() {
            cmd.entity(e).despawn();
        }
    });
}

fn update_sat_pos(
    mut sats: Query<(
        &TLETimeStamp,
        &SGP4Constants,
        &mut TEMEPos,
        &mut TEMEVelocity,
    )>,
) {
    sats.for_each_mut(|(ts, constants, mut pos, mut vel)| {
        (*pos, *vel) = propagate_sat(ts.0 as f64, &constants.0);
    });
}
fn update_lonlat(mut cmd: Commands, sats: Query<(Entity, &TEMEPos), Changed<TEMEPos>>) {
    let datetime: DateTime<Utc> = Utc::now();
    sats.for_each(|(e, pos)| {
        let (x, y, z) = map_3d::eci2ecef(
            map_3d::utc2gst([
                datetime.year() as i32,
                datetime.month() as i32,
                datetime.day() as i32,
                datetime.hour() as i32,
                datetime.minute() as i32,
                datetime.second() as i32,
            ]),
            pos.0[0] * 1000.0,
            pos.0[1] * 1000.0,
            pos.0[2] * 1000.0,
        );
        let (x, y, z) = map_3d::ecef2geodetic(x, y, z, map_3d::Ellipsoid::WGS84);
        let res = (map_3d::rad2deg(x), map_3d::rad2deg(y), z / 1000.0);
        cmd.entity(e).insert(LatLonAlt(res));
    });
}

fn update_every_sat(mut cmd: Commands, satdata: Res<SatInfo>, sats: Query<(Entity, &SatID)>) {
    if satdata.is_changed() {
        sats.for_each(|(e, id)| {
            if !satdata.sats.contains_key(&id.0) {
                cmd.entity(e).despawn_recursive();
            } else {
                let s = satdata.sats.get(&id.0).unwrap();
                let constants = sgp4::Constants::from_elements(s).unwrap();
                cmd.entity(e).insert(SGP4Constants(constants));
                cmd.entity(e).insert(TLETimeStamp(s.datetime.timestamp()));
                cmd.entity(e).insert(Name::from(get_name(&satdata, &id)));
            }
        });
    }
}

pub fn init_sat_data(mut cmd: Commands, rt: Res<Runtime>) {
    let s = rt.block_on(get_sat_data()).unwrap();
    let mut sat_info = SatInfo::default();
    for elements in s {
        if elements.object_name.as_ref().unwrap().contains(&"STARLINK") {
            sat_info.sats.insert(elements.norad_id, elements);
        }
    }

    for (_k, elements) in &sat_info.sats {
        let id = SatID(elements.norad_id);

        let constants = sgp4::Constants::from_elements(elements).unwrap();
        let ts = TLETimeStamp(elements.datetime.timestamp());
        let (pos, vel) = propagate_sat(ts.0 as f64, &constants);
        cmd.spawn().insert_bundle((
            id,
            SGP4Constants(constants),
            ts,
            pos,
            vel,
            Name::from(elements.object_name.as_ref().unwrap().clone()),
        ));
    }
    cmd.insert_resource(sat_info);
}

fn propagate_sat(tlets: f64, constants: &Constants) -> (TEMEPos, TEMEVelocity) {
    let ts = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    let ts = ts - tlets;
    let prediction = constants.propagate(ts / 60.0).unwrap();
    let (pos, vel) = (
        TEMEPos(prediction.position),
        TEMEVelocity(prediction.velocity),
    );

    (pos, vel)
}

#[derive(Default)]
pub struct SGP4Plugin;

impl Plugin for SGP4Plugin {
    fn build(&self, app: &mut App) {
        let rt = Runtime::new().unwrap();
        app.insert_resource(QueryConfig {
            timer: Timer::new(Duration::from_secs(60 * 24 * 24), true),
        });
        app.insert_resource(rt);
        app.insert_resource(SatInfo::default());
        app.add_startup_system(init_sat_data);
        app.add_system_to_stage(CoreStage::PreUpdate, update_data);
        app.add_system_to_stage(CoreStage::Update, receive_task);
        app.add_system_to_stage(CoreStage::Update, update_every_sat.after(receive_task));
        app.add_system_to_stage(CoreStage::Update, update_sat_pos.after(update_every_sat));
        app.add_system_to_stage(CoreStage::Update, update_lonlat);
    }
}
