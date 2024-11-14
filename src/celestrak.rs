//7R7C44-NVYFK3-9VFVKJ-4X3J
//"https://api.n2yo.com/rest/v1/satellite/"
//tle/52997&apiKey=

use bevy::prelude::*;
use tokio::runtime::Builder;

use std::{
    collections::HashMap,
    fs::OpenOptions,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use chrono::{DateTime, Timelike, Utc};
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime};

use serde::{Deserialize, Serialize};
use sgp4::{Constants, Elements, MinutesSinceEpoch};

//https://celestrak.org/NORAD/elements/gp.php?GROUP=STARLINK&FORMAT=TLE
pub(crate) async fn get_online_sat_data() -> Result<Vec<sgp4::Elements>, reqwest::Error> {
    let resp =//?GROUP=STARLINK
        reqwest::get("https://celestrak.org/NORAD/elements/gp.php?GROUP=STARLINK&FORMAT=JSON")
            .await?
            .json::<Vec<sgp4::Elements>>()
            .await;
    resp
}
#[derive(Component, serde::Serialize, serde::Deserialize)]
pub struct CElements(sgp4::Elements);
#[derive(Default, Component)]
pub struct SatName(pub String);

#[derive(Default, Component)]
pub struct SatID(pub u64);
#[derive(Component)]
pub struct TaskWrapper<T>(pub Option<tokio::task::JoinHandle<T>>);
#[derive(Resource)]
pub struct QueryConfig {
    pub timer: Timer,
}
#[derive(Resource)]
pub struct TLECacheConfig {
    pub file: PathBuf,
    pub cache: Option<Vec<Elements>>,
}

#[derive(Default, Component)]
pub struct TEMEPos(pub [f64; 3]);
#[derive(Default, Component)]
pub struct TEMEVelocity(pub [f64; 3]);

#[derive(Component)]
pub struct SGP4Constants(pub Constants);

#[derive(Component)]
pub struct LatLonAlt(pub (f64, f64, f64));

#[derive(Component)]
pub struct TLETimeStamp(pub NaiveDateTime);
#[derive(Default, Serialize, Deserialize, Resource)]
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
#[derive(Resource)]
pub struct Runtime(pub tokio::runtime::Runtime);
fn update_data(
    mut cmd: Commands,
    rt: Res<Runtime>,
    mut config: ResMut<QueryConfig>,
    time: Res<bevy::time::Time>,
) {
    config.timer.tick(time.delta());

    if config.timer.finished() {
        let task = rt.0.spawn(async { get_online_sat_data().await });
        cmd.spawn(TaskWrapper(Some(task)));
        // info!(
        //     "Query the satellite info at {}",
        //     time.seconds_since_startup()
        // );
    }
}
fn receive_task(
    mut cmd: Commands,
    rt: Res<Runtime>,
    mut tasks: Query<(
        Entity,
        &mut TaskWrapper<Result<Vec<Elements>, reqwest::Error>>,
    )>,
    mut sat: ResMut<SatInfo>,
) {
    tasks.iter_mut().for_each(|(e, mut t)| {
        if t.0.is_some() {
            if t.0.as_ref().unwrap().is_finished() {
                let s = t.0.take().unwrap();
                let res = rt.0.block_on(s).unwrap();
                match res {
                    Ok(res) => {
                        let mut sat_info = SatInfo::default();
                        for elements in res {
                            sat_info.sats.insert(elements.norad_id, elements);
                        }
                        *sat = sat_info;
                        info!("Meassge Received! {}", sat.sats.len());
                    }
                    Err(err) => {
                        error!("Update TLE failed! {}", err);
                    }
                }
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
        &Name,
    )>,
) {
    use std::time::Instant;
    let _now = Instant::now();
    sats.iter_mut().for_each(|(ts, constants, mut pos, mut vel, n)| {
        if let Ok((p, v)) = propagate_sat(&ts.0, &constants.0) {
            *pos = p;
            *vel = v;
        } else {
            error!("{} diverged", n.as_str());
        }
    });
}

fn ecef_to_wgs84(x: f64, y: f64, z: f64) -> (f64, f64, f64) {
    let a: f64 = 6_378_137.0;
    let f: f64 = 1.0 / 298.257223563;
    let b = a * (1.0 - f);
    let e_sq = 2.0 * f - f.powi(2);
    let ep_sq = (a.powi(2) - b.powi(2)) / a.powi(2);
    // Calculate longitude
    let longitude = y.atan2(x);

    // Calculate intermediate values
    let p = (x.powi(2) + y.powi(2)).sqrt();
    let phi = (z / (p * (1.0 - ep_sq))).atan();
    //let phi = (z + eta * b * (beta.sin().powi(3))) / (p - ep_sq * a * (beta.cos().powi(3)));
    // Calculate latitude
    let latitude = phi;

    // Calculate altitude
    let v = 1.0 / (1.0 - e_sq * (latitude.sin().powi(2))).sqrt();
    let altitude = p * latitude.cos() + z * latitude.sin() - a / v;

    (latitude, longitude, altitude)
}

fn ecef_to_geodetic(x: f64, y: f64, z: f64) -> (f64, f64, f64) {
    let a: f64 = 6378137.0; // Semi-major axis for WGS84
    let e_squared: f64 = 0.00669437999014; // Square of the first eccentricity for WGS84
    let e2: f64 = e_squared / (1.0 - e_squared);

    let p = (x * x + y * y).sqrt();
    let theta = (z / p * (1.0 - e_squared)).atan();

    let sin_theta = theta.sin();
    let cos_theta = theta.cos();

    let num = z + e2 * a * sin_theta.powi(3);
    let den = p - e_squared * a * cos_theta.powi(3);

    let phi = num.atan2(den);
    let lambda = y.atan2(x);
    let sin_phi = phi.sin();
    let n = a / (1.0 - e_squared * sin_phi * sin_phi).sqrt();
    let h = p * phi.cos() + z * sin_phi - a * a / n;
    (phi, lambda, h)
}
fn update_lonlat(mut cmd: Commands, sats: Query<(Entity, &TEMEPos), Changed<TEMEPos>>) {
    let datetime: DateTime<Utc> = Utc::now();
    sats.iter().for_each(|(e, pos)| {
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
        let (x, y, z) = ecef_to_wgs84(x, y, z);
        //let (x, y, z) = map_3d::ecef2geodetic(x, y, z, map_3d::Ellipsoid::WGS84);
        let res = (map_3d::rad2deg(x), map_3d::rad2deg(y), z / 1000.0);
        cmd.entity(e).insert(LatLonAlt(res));
    });
}

fn update_every_sat(mut cmd: Commands, satdata: Res<SatInfo>, sats: Query<(Entity, &SatID)>) {
    if satdata.is_changed() {
        sats.iter().for_each(|(e, id)| {
            if !satdata.sats.contains_key(&id.0) {
                cmd.entity(e).despawn_recursive();
            } else {
                let s = satdata.sats.get(&id.0).unwrap();
                let constants = sgp4::Constants::from_elements(s).unwrap();
                cmd.entity(e).insert(SGP4Constants(constants));
                cmd.entity(e).insert(TLETimeStamp(s.datetime));
                cmd.entity(e).insert(Name::from(get_name(&satdata, &id)));
            }
        });
    }
}

pub fn init_sat_data(
    mut cmd: Commands,
    mut cache: ResMut<TLECacheConfig>,
    timer: Res<QueryConfig>,
    rt: Res<Runtime>,
) {
    //attempt read from file
    let file = std::fs::File::open(cache.file.clone());
    let mut tle = None;
    let read_online = match file {
        Ok(f) => {
            let j = serde_json::from_reader::<_, Vec<Elements>>(f);
      
            match j {
             
                Ok(j) => {
                    let sampled_time = j[0].datetime;
                    let t = Utc::now().naive_utc();
                    let dt = t - sampled_time;
                    let d = timer.timer.duration();

                    if dt.to_std().unwrap() > 7 * d {
                        true
                    } else {
                        tle = Some(j);
                        false
                    }
                }
                Err(err) => {
                    error!("cannot read {:?}: {}!", cache.file, err);
                    true
                }
            }
        }
        Err(err) => {
            error!("cannot open {:?}: {}!", cache.file, err);
            true
        }
    };
    println!("j:{:?}", read_online);
    if read_online {
        //read from online
        let data = rt.0.block_on(async { get_online_sat_data().await });
        tle = match data {
            Ok(data) => Some(data),
            Err(err) => {
                error!("cannot read tle from online {}!", err);
                tle
            }
        };
        if tle.is_some() {
            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(cache.file.clone()).unwrap();
            file.write(
                serde_json::to_string(tle.as_ref().unwrap())
                    .unwrap()
                    .as_bytes(),
            )
            .unwrap();
        }
    }
    cache.cache = tle;
    let s = cache.cache.as_ref().unwrap();
    let js: Vec<Elements> = serde_json::from_value(serde_json::to_value(s).unwrap()).unwrap();
    let mut sat_info = SatInfo::default();
    for elements in js {
        sat_info.sats.insert(elements.norad_id, elements);
    }
    // for elements in s {
    //     if elements.object_name.as_ref().unwrap().contains(&"STARLINK") {
    //         sat_info.sats.insert(elements.norad_id, elements);
    //     }
    // }

    for (_k, elements) in &sat_info.sats {
        let id = SatID(elements.norad_id);

        let constants = sgp4::Constants::from_elements(elements).unwrap();

        let ts = TLETimeStamp(elements.datetime);
        if let Ok((pos, vel)) = propagate_sat(&ts.0, &constants) {
            cmd.spawn((
                id,
                SGP4Constants(constants),
                ts,
                pos,
                vel,
                Name::from(elements.object_name.as_ref().unwrap().clone()),
            ));
        } else {
            error!("{} diverged", elements.object_name.as_ref().unwrap());
            cmd.spawn((
                id,
                SGP4Constants(constants),
                ts,
                Name::from(elements.object_name.as_ref().unwrap().clone()),
            ));
        }
    }
    cmd.insert_resource(sat_info);
}

fn propagate_sat(
    init_ts: &NaiveDateTime,
    constants: &Constants,
) -> Result<(TEMEPos, TEMEVelocity), ()> {
    let ts = chrono::Utc::now();
    let ts = ts.naive_utc() - *init_ts;
    if let Ok(prediction) = constants.propagate(sgp4::MinutesSinceEpoch(
        ts.to_std().unwrap().as_secs_f64() / 60.0,
    )) {
        let (pos, vel) = (
            TEMEPos(prediction.position),
            TEMEVelocity(prediction.velocity),
        );

        Ok((pos, vel))
    } else {
        Err(())
    }
}

#[derive(Default)]
pub struct SGP4Plugin;

impl Plugin for SGP4Plugin {
    fn build(&self, app: &mut App) {
        let rt = Runtime(
            Builder::new_multi_thread()
                .enable_all()
                .worker_threads(4)
                .build()
                .unwrap(),
        );
        app.insert_resource(TLECacheConfig {
            file: "./tle.json".into(),
            cache: None,
        });
        app.insert_resource(QueryConfig {
            timer: Timer::new(Duration::from_secs(60 * 24 * 24), TimerMode::Repeating),
        });
        app.insert_resource(rt);
        app.insert_resource(SatInfo::default());
        app.add_systems(Startup,init_sat_data);
        app.add_systems(PreUpdate,update_data);
        app.add_systems(Update,
            (receive_task, update_every_sat, update_sat_pos)
                .chain()
        );

        app.add_systems(Update,update_lonlat);
    }
}
