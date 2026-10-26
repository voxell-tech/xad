use bevy::prelude::*;
use bezier::{
    BezierCurve, TileGrid, bounding_box, detect_faces,
    split_intersections,
};
use sketch::Sketch;

use crate::bounds::Bounds;

// Margin around the face's curves. The shader's distance bound outside
// the grid must stay positive at the grid edge, so the grid has to
// extend past every curve.
const GRID_PAD: f32 = 0.05;
// Tiles along the profile's longer axis.
// TODO: tune from curve count/density instead of a constant.
const GRID_RESOLUTION: u32 = 16;

/// How far a face is swept along its plane normal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Extent {
    pub min: f32,
    pub max: f32,
}

impl Extent {
    /// Rises from the sketch plane.
    pub fn blind(depth: f32) -> Self {
        Self {
            min: 0.0,
            max: depth,
        }
    }

    /// Straddles the sketch plane.
    pub fn symmetric(depth: f32) -> Self {
        Self {
            min: -depth * 0.5,
            max: depth * 0.5,
        }
    }
}

/// A sketch plane's world origin and axes.
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub origin: Vec3,
    pub tangent: Vec3,
    pub bitangent: Vec3,
    pub normal: Vec3,
}

impl Frame {
    pub fn of(sketch: &Sketch) -> Self {
        let (tangent, bitangent) = sketch.plane_basis();
        Self {
            origin: Vec3::ZERO,
            tangent,
            bitangent,
            normal: sketch.plane.normal(),
        }
    }

    /// Plane coordinates of a world point, `z` along the normal.
    pub fn to_local(&self, world: Vec3) -> Vec3 {
        let rel = world - self.origin;
        Vec3::new(
            rel.dot(self.tangent),
            rel.dot(self.bitangent),
            rel.dot(self.normal),
        )
    }

    pub fn to_world(&self, local: Vec3) -> Vec3 {
        self.origin
            + self.tangent * local.x
            + self.bitangent * local.y
            + self.normal * local.z
    }
}

/// One sketch face swept along its plane normal.
#[derive(Clone, Debug)]
pub struct Solid {
    /// The face boundary, every curve oriented with the face on its
    /// left, so a nearest-curve query is negative inside the face.
    pub curves: Vec<BezierCurve>,
    pub grid: TileGrid,
    pub extent: Extent,
    pub frame: Frame,
}

impl Solid {
    /// `face` indexes [`detect_faces`] on the sketch's curves.
    ///
    /// TODO: the caller re-splits and re-detects per face; cache both on
    /// the sketch once face detection is driven by change detection.
    pub fn from_face(
        sketch: &Sketch,
        face: usize,
        extent: Extent,
    ) -> Option<Self> {
        let split = split_intersections(sketch.curves.clone());
        let detected = detect_faces(&split);

        // Curves bounding `face` on either side, flipped when it sits
        // on the right so the face always ends up on the left. Taking
        // both sides keeps hole loops, which Face::boundary drops.
        let curves = split
            .iter()
            .zip(&detected.polarity)
            .filter_map(|(curve, polarity)| {
                let mut curve = *curve;
                if polarity.right == Some(face) {
                    std::mem::swap(&mut curve.p0, &mut curve.p2);
                } else if polarity.left != Some(face) {
                    return None;
                }
                Some(curve)
            })
            .collect::<Vec<_>>();

        let profile = bounding_box(&curves)?.padded(GRID_PAD);
        Some(Self {
            grid: TileGrid::build(&curves, profile, GRID_RESOLUTION),
            curves,
            extent,
            frame: Frame::of(sketch),
        })
    }

    pub fn bounds(&self) -> Bounds {
        let profile = self.grid.bounds;
        let corners = [
            Vec2::new(profile.min.x, profile.min.y),
            Vec2::new(profile.max.x, profile.min.y),
            Vec2::new(profile.max.x, profile.max.y),
            Vec2::new(profile.min.x, profile.max.y),
        ];
        Bounds::of(corners.into_iter().flat_map(|c| {
            [self.extent.min, self.extent.max]
                .map(|z| self.frame.to_world(Vec3::new(c.x, c.y, z)))
        }))
        .expect("corners are never empty")
    }
}
