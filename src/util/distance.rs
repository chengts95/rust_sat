use map_3d::{deg2rad, EARTH_RADIUS};

/// Returns distance (m) between two decimal degrees coordinates::
/// coord1: (lat,lon), coord2: (lat, lon)
pub fn distance(coord1: (f64, f64), coord2: (f64, f64)) -> f64 {
    let dphi = deg2rad(coord2.0) - deg2rad(coord1.0);
    let d_lambda = deg2rad(coord2.1) - deg2rad(coord1.1);
    let a: f64 = (dphi / 2.0_f64).sin().powi(2)
        + deg2rad(coord1.0).cos() * deg2rad(coord2.0).cos() * (d_lambda / 2.0_f64).sin().powi(2);
    let c = 2.0_f64 * a.sqrt().atan2((1.0 - a).sqrt());
    EARTH_RADIUS * c
}

/// Returns radians between two decimal degrees coordinates::
/// coord1: (lat,lon), coord2: (lat, lon)
pub fn geodegree(coord1: (f64, f64), coord2: (f64, f64)) -> f64 {
    let dphi = deg2rad(coord2.0) - deg2rad(coord1.0);
    let d_lambda = deg2rad(coord2.1) - deg2rad(coord1.1);
    let a: f64 = (dphi / 2.0_f64).sin().powi(2)
        + deg2rad(coord1.0).cos() * deg2rad(coord2.0).cos() * (d_lambda / 2.0_f64).sin().powi(2);
    let c = 2.0_f64 * a.sqrt().atan2((1.0 - a).sqrt());
    c
}

#[inline(always)]
pub fn trangle_distance(a: f64, b: f64, theta: f64) -> f64 {
    let c = (a.powf(2.0) + b.powf(2.0) - 2.0 * a * b * theta.cos()).sqrt();
    c
}

pub fn ground_space_distance(ground_station: (f64, f64), satellite: (f64, f64, f64)) -> f64 {
    let theta_radians = geodegree(ground_station, (satellite.0, satellite.1));
    let sigma_radians = 0.0_f64;
    let h = satellite.2;
    let r = EARTH_RADIUS;
    let f1 = ((h + r) / r).powi(2) - (sigma_radians).cos().powi(2);
    let mut d: f64 = f1.sqrt() - (sigma_radians).sin();
    d = r * d;
    let h_gs = d * sigma_radians.sin();

    trangle_distance(h + r, h_gs + r, theta_radians)
}
