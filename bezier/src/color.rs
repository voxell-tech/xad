use bevy_color::LinearRgba;
use rand::seq::IndexedRandom;

// TODO: consider throwing this into some sort of config instead of hardcoding it
const PALETTE: [LinearRgba; 7] = [
    // random colours
    LinearRgba::new(1.00, 0.55, 0.10, 1.0), // orange
    LinearRgba::new(0.20, 0.80, 1.00, 1.0), // cyan
    LinearRgba::new(1.00, 0.25, 0.45, 1.0), // pink
    LinearRgba::new(0.55, 1.00, 0.35, 1.0), // green
    LinearRgba::new(1.00, 0.85, 0.10, 1.0), // yellow
    LinearRgba::new(0.90, 0.40, 1.00, 1.0), // purple
    LinearRgba::new(1.00, 0.30, 0.30, 1.0), // red
];

pub fn gen_color() -> LinearRgba {
    *PALETTE.choose(&mut rand::rng()).unwrap()
}
