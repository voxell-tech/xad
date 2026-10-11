use bevy_color::LinearRgba;
use rand::seq::IndexedRandom;

// TODO: consider throwing this into some sort of config instead of hardcoding it
const PALETTE: [LinearRgba; 16] = [
    // random colours
    LinearRgba::new(1.00, 0.55, 0.10, 1.0), // orange
    LinearRgba::new(0.20, 0.80, 1.00, 1.0), // cyan
    LinearRgba::new(1.00, 0.25, 0.45, 1.0), // pink
    LinearRgba::new(0.55, 1.00, 0.35, 1.0), // green
    LinearRgba::new(1.00, 0.85, 0.10, 1.0), // yellow
    LinearRgba::new(0.90, 0.40, 1.00, 1.0), // purple
    LinearRgba::new(1.00, 0.30, 0.30, 1.0), // red
    LinearRgba::new(0.40, 0.60, 1.00, 1.0), // blue
    LinearRgba::new(0.10, 0.90, 0.70, 1.0), // teal
    LinearRgba::new(0.75, 0.90, 0.20, 1.0), // lime
    LinearRgba::new(1.00, 0.45, 0.70, 1.0), // rose
    LinearRgba::new(0.65, 0.50, 1.00, 1.0), // indigo
    LinearRgba::new(1.00, 0.70, 0.30, 1.0), // amber
    LinearRgba::new(0.30, 0.95, 0.45, 1.0), // emerald
    LinearRgba::new(0.95, 0.35, 0.75, 1.0), // magenta
    LinearRgba::new(0.50, 0.85, 0.95, 1.0), // sky
];

pub fn gen_color() -> LinearRgba {
    *PALETTE.choose(&mut rand::rng()).unwrap()
}
