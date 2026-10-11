use bevy_color::LinearRgba;
use bevy_math::{Vec2, Vec4};
use encase::ShaderType;

#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct BezierCurve {
    // Control points
    pub p0: Vec2,
    pub p1: Vec2,
    pub p2: Vec2,

    pub color: Vec4,

    // Half-width
    pub width: f32,

    // 0: Dont show
    // 1: Show control points
    pub debug: u32,

    // 0: Dont show
    // 1: Show a dot at each endpoint (p0 and p2)
    pub draw_endpoints: u32,

    // Radius of the endpoint dots
    pub endpoint_radius: f32,
}

impl BezierCurve {
    pub fn new(p0: Vec2, p1: Vec2, p2: Vec2) -> Self {
        Self {
            p0,
            p1,
            p2,
            color: Vec4::ONE,
            width: 0.012,
            debug: 0,
            draw_endpoints: 0,
            endpoint_radius: 0.02,
        }
    }

    pub fn with_color(mut self, color: LinearRgba) -> Self {
        self.color = Vec4::new(
            color.red,
            color.green,
            color.blue,
            color.alpha,
        );
        self
    }

    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug as u32;
        self
    }

    pub fn with_draw_endpoints(
        mut self,
        draw_endpoints: bool,
    ) -> Self {
        self.draw_endpoints = draw_endpoints as u32;
        self
    }

    pub fn with_endpoint_radius(
        mut self,
        endpoint_radius: f32,
    ) -> Self {
        self.endpoint_radius = endpoint_radius;
        self
    }

    // evaluate the quadratic curve at `t`,` in [0, 1]
    pub fn eval(&self, t: f32) -> Vec2 {
        let a = self.p0.lerp(self.p1, t);
        let b = self.p1.lerp(self.p2, t);
        a.lerp(b, t)
    }
}
