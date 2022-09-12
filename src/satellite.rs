use bevy::prelude::*;
use derive_more::{Add, Display, From, Into};
use serde::{Deserialize, Serialize};
#[derive(Default, Component, From, Into)]
pub struct SatName(pub String);

#[derive(Default, Component, From, Into)]
pub struct Name(pub String);

#[derive(Default, Component, From, Into)]
pub struct SatID(pub i64);
#[derive(Default, Component)]
pub struct Coord {
    pub lat: f32,
    pub lng: f32,
}
#[derive(Default, Component)]
pub struct Coord2 {
    lat: f32,
    lng: f32,
}
#[derive(Default, Component, From, Into)]
pub struct Altitude(pub f32);
#[derive(Default, Component, From, Into)]
pub struct Illuminated(pub bool);
#[derive(Default, Component, From, Into)]
pub struct ServedDays(pub f32);

// right ascension of the ascending node (RAAN).
#[derive(PartialEq,Default, Component, From, Into)]
pub struct RAAN(pub f32);

//Perigee height
#[derive(Default, Component, From, Into)]
pub struct Perigee(pub f32);

#[derive(Default, Component, From, Into)]
pub struct Altitude2(pub f32);

#[derive(Clone,Component, Default, Serialize, Deserialize, Debug)]
pub struct Satellite {
    pub id: i64,
    pub name: String,
    pub oname: String,
    pub lat: f32,
    pub lng: f32,
    pub alt: f32,
    pub alt2: f32,
    pub p: f32,
    pub lat2: f32,
    pub lng2: f32,
    pub illum: i32,
    pub raan: f32,
    pub age: f32,
}

#[derive(Bundle, Default)]
pub struct SatelliteBundle {
    pub id: SatID,
    pub name: Name,
    pub oname: SatName,
    pub coord: Coord,
    pub alt: Altitude,
    pub alt2: Altitude2,
    pub p: Perigee,
    pub coord2: Coord2,
    pub illum: Illuminated,
    pub raan: RAAN,
    pub age: ServedDays,
}
impl From<Satellite> for SatelliteBundle {
    fn from(s: Satellite) -> Self {
        Self {
            id: s.id.into(),
            name: s.name.into(),
            oname: s.oname.into(),
            coord: Coord {
                lat: s.lat,
                lng: s.lng,
            },
            alt: s.alt.into(),
            alt2: s.alt2.into(),
            p: s.p.into(),
            coord2: Coord2 {
                lat: s.lat2,
                lng: s.lng2,
            },
            illum: Illuminated(s.illum !=0),
            raan: s.raan.into(),
            age: s.age.into(),
        }
    }
}
