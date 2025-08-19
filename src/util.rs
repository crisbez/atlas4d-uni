
use chrono::{DateTime, Utc};
use std::f64::consts::PI;

pub fn haversine_m(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6_371_000.0_f64;
    let d2r = PI / 180.0_f64;
    let (phi1, phi2) = (lat1 * d2r, lat2 * d2r);
    let dphi = (lat2 - lat1) * d2r;
    let dlambda = (lon2 - lon1) * d2r;
    let a = (dphi / 2.0).sin().powi(2) + phi1.cos() * phi2.cos() * (dlambda / 2.0).sin().powi(2);
    2.0 * r * a.sqrt().asin()
}

pub fn time_bin_5min(t: &DateTime<Utc>) -> i64 {
    let s = t.timestamp();
    s - (s % (5 * 60))
}
