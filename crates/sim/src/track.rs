//! Tracks: a ground profile plus the gameplay markers placed along it.
//!
//! The ground is a polyline running left to right with solid ground below it. It is
//! kept a function of x (x strictly increases) so height lookups are unambiguous and a
//! future track editor can stay simple enough for a 12-year-old.

use serde::{Deserialize, Serialize};

use crate::math::{atan2, Vector};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZoneKind {
    /// A labelled section, shown as a sign.
    Section,
    /// A long flat strip with distance markers for wheelie practice.
    WheelieStrip,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Zone {
    pub kind: ZoneKind,
    pub label: String,
    pub x0: f32,
    pub x1: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub name: String,
    /// Ground profile `[x, y]`, left to right, x strictly increasing.
    pub ground: Vec<[f32; 2]>,
    /// Respawn points (x). The first one is the start.
    pub checkpoints: Vec<f32>,
    /// Crossing this x finishes a run.
    pub finish_x: f32,
    /// Falling below this height counts as a crash.
    pub kill_y: f32,
    pub zones: Vec<Zone>,
}

impl Track {
    pub fn start_x(&self) -> f32 {
        self.checkpoints.first().copied().unwrap_or(0.0)
    }

    /// Ground height at `x` (clamped to the ends of the track).
    pub fn ground_y(&self, x: f32) -> f32 {
        let g = &self.ground;
        if g.is_empty() {
            return 0.0;
        }
        if x <= g[0][0] {
            return g[0][1];
        }
        if x >= g[g.len() - 1][0] {
            return g[g.len() - 1][1];
        }
        // First point with p.x > x; the segment is [i - 1, i].
        let i = g.partition_point(|p| p[0] <= x);
        let (a, b) = (g[i - 1], g[i]);
        let t = (x - a[0]) / (b[0] - a[0]);
        a[1] + (b[1] - a[1]) * t
    }

    /// Ground slope angle at `x`, measured over `span` metres.
    pub fn ground_angle(&self, x: f32, span: f32) -> f32 {
        let h = span * 0.5;
        atan2(self.ground_y(x + h) - self.ground_y(x - h), span)
    }

    /// Index of the last checkpoint at or behind `x`.
    pub fn checkpoint_index_at(&self, x: f32) -> usize {
        self.checkpoints.iter().rposition(|&c| c <= x).unwrap_or(0)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("track serializes")
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        let track: Track = serde_json::from_str(json).map_err(|e| e.to_string())?;
        track.validate()?;
        Ok(track)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.ground.len() < 2 {
            return Err("track needs at least two ground points".into());
        }
        if self.ground.windows(2).any(|w| w[1][0] <= w[0][0]) {
            return Err("ground x must strictly increase".into());
        }
        if self.ground.iter().flatten().any(|v| !v.is_finite()) {
            return Err("ground has non-finite points".into());
        }
        if self.checkpoints.is_empty() {
            return Err("track needs a start checkpoint".into());
        }
        Ok(())
    }

    /// The prototype park: one of everything the riding prototype needs to test.
    pub fn proving_grounds() -> Track {
        use ZoneKind::*;
        TrackBuilder::new("Proving Grounds", -30.0, 0.0)
            .wall(8.0)
            .flat(30.0)
            .checkpoint()
            .flat(15.0)
            .section(Section, "Rollers", |b| b.rollers(4, 7.0, 0.55).flat(12.0))
            .checkpoint()
            .flat(6.0)
            .section(WheelieStrip, "Wheelie Strip", |b| b.flat(70.0))
            .flat(10.0)
            .checkpoint()
            .flat(12.0)
            .section(Section, "Kicker", |b| {
                b.curve(6.0, 1.3, 0.6).line(1.0, -1.3).flat(26.0)
            })
            .checkpoint()
            .flat(14.0)
            .section(Section, "Tabletop", |b| {
                b.curve(8.0, 2.0, 0.58)
                    .corner(0.0)
                    .flat(7.0)
                    .curve(10.0, -2.0, 0.0)
                    .flat(18.0)
            })
            .checkpoint()
            .flat(32.0)
            .section(Section, "Gap", |b| {
                b.curve(9.0, 2.2, 0.7)
                    .line(1.2, -5.2)
                    .flat(3.6)
                    .line(1.2, 5.2)
                    .corner(0.0)
                    .curve(12.0, -2.2, 0.0)
                    .flat(20.0)
            })
            .checkpoint()
            .flat(12.0)
            .section(Section, "Hill Climb", |b| {
                b.curve(12.0, 3.0, 0.5)
                    .line(9.0, 4.5)
                    .curve(7.0, 1.6, 0.0)
                    .flat(5.0)
                    .curve(12.0, -4.0, -0.55)
                    .line(7.0, -3.85)
                    .curve(9.0, -1.25, 0.0)
                    .flat(16.0)
            })
            .checkpoint()
            .flat(10.0)
            .section(Section, "Steps", |b| {
                b.line(0.3, 0.25)
                    .flat(3.5)
                    .line(0.3, 0.25)
                    .flat(3.5)
                    .line(0.3, 0.25)
                    .flat(4.0)
                    .corner(0.0)
                    .curve(7.0, -0.75, 0.0)
                    .flat(18.0)
            })
            .checkpoint()
            .flat(40.0)
            .section(Section, "Big Air", |b| {
                b.curve(16.0, 3.8, 0.8)
                    .line(1.0, -3.8)
                    .flat(4.0)
                    .line(1.0, 3.2)
                    .corner(0.0)
                    .curve(24.0, -3.2, 0.0)
                    .flat(20.0)
            })
            .checkpoint()
            .flat(25.0)
            .finish()
            .flat(45.0)
            .wall(8.0)
            .build()
    }
}

/// Builds a track from pieces. Curves are cubic in x, so the ground stays a function of
/// x, and each piece starts with the slope the previous piece ended on (C1 continuous)
/// unless `corner` resets it.
pub struct TrackBuilder {
    name: String,
    pts: Vec<Vector>,
    slope: f32,
    checkpoints: Vec<f32>,
    finish_x: Option<f32>,
    zones: Vec<Zone>,
}

/// Horizontal sample spacing for curves (m).
const SAMPLE_STEP: f32 = 0.25;

impl TrackBuilder {
    pub fn new(name: &str, x: f32, y: f32) -> Self {
        Self {
            name: name.into(),
            pts: vec![Vector::new(x, y)],
            slope: 0.0,
            checkpoints: Vec::new(),
            finish_x: None,
            zones: Vec::new(),
        }
    }

    fn cur(&self) -> Vector {
        *self.pts.last().expect("builder always has a point")
    }

    fn push(&mut self, p: Vector) {
        if p.x > self.cur().x {
            self.pts.push(p);
        }
    }

    /// Straight line to a point `dx, dy` ahead.
    pub fn line(mut self, dx: f32, dy: f32) -> Self {
        let c = self.cur();
        self.push(Vector::new(c.x + dx, c.y + dy));
        self.slope = dy / dx;
        self
    }

    pub fn flat(self, len: f32) -> Self {
        self.line(len, 0.0)
    }

    /// Near-vertical wall (still a function of x) to keep riders on the track.
    pub fn wall(self, height: f32) -> Self {
        self.line(0.3, height).line(0.3, -height).corner(0.0)
    }

    /// Resets the slope the next curve starts from, making a sharp corner.
    pub fn corner(mut self, slope: f32) -> Self {
        self.slope = slope;
        self
    }

    /// Smooth cubic curve to a point `dx, dy` ahead, arriving with slope `end_slope`.
    pub fn curve(mut self, dx: f32, dy: f32, end_slope: f32) -> Self {
        let c = self.cur();
        let s0 = self.slope;
        let n = ((dx.max(dy.abs())) / SAMPLE_STEP).ceil().max(2.0) as usize;
        for i in 1..=n {
            let t = i as f32 / n as f32;
            let (t2, t3) = (t * t, t * t * t);
            let h10 = t3 - 2.0 * t2 + t;
            let h01 = -2.0 * t3 + 3.0 * t2;
            let h11 = t3 - t2;
            let y = h10 * dx * s0 + h01 * dy + h11 * dx * end_slope;
            self.push(Vector::new(c.x + t * dx, c.y + y));
        }
        self.slope = end_slope;
        self
    }

    /// `n` smooth bumps of the given wavelength and height.
    pub fn rollers(mut self, n: usize, wavelength: f32, height: f32) -> Self {
        let c = self.cur();
        let steps = ((n as f32 * wavelength) / SAMPLE_STEP).ceil() as usize;
        for i in 1..=steps {
            let x = i as f32 * SAMPLE_STEP;
            let phase = crate::math::TAU * x / wavelength;
            let y = 0.5 * height * (1.0 - crate::math::cos(phase));
            self.push(Vector::new(c.x + x, c.y + y));
        }
        self.slope = 0.0;
        self
    }

    pub fn checkpoint(mut self) -> Self {
        let x = self.cur().x;
        self.checkpoints.push(x);
        self
    }

    pub fn finish(mut self) -> Self {
        self.finish_x = Some(self.cur().x);
        self
    }

    /// Wraps the pieces added by `f` in a labelled zone.
    pub fn section(self, kind: ZoneKind, label: &str, f: impl FnOnce(Self) -> Self) -> Self {
        let x0 = self.cur().x;
        let mut b = f(self);
        let x1 = b.cur().x;
        b.zones.push(Zone {
            kind,
            label: label.into(),
            x0,
            x1,
        });
        b
    }

    pub fn build(mut self) -> Track {
        let end_x = self.cur().x;
        if self.checkpoints.is_empty() {
            self.checkpoints.push(self.pts[0].x);
        }
        let track = Track {
            name: self.name,
            ground: self.pts.iter().map(|p| [p.x, p.y]).collect(),
            checkpoints: self.checkpoints,
            finish_x: self.finish_x.unwrap_or(end_x),
            kill_y: -12.0,
            zones: self.zones,
        };
        debug_assert!(track.validate().is_ok());
        track
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proving_grounds_is_valid() {
        let t = Track::proving_grounds();
        t.validate().unwrap();
        assert!(t.checkpoints.len() >= 8);
        assert!(t.finish_x > t.start_x());
        assert!(t.zones.iter().any(|z| z.kind == ZoneKind::WheelieStrip));
        // Every checkpoint sits on flat ground so respawns are calm.
        for &c in &t.checkpoints {
            assert!(
                t.ground_angle(c, 2.0).abs() < 0.05,
                "checkpoint at {c} is on a slope"
            );
        }
    }

    #[test]
    fn ground_lookup_interpolates() {
        let t = TrackBuilder::new("t", 0.0, 0.0).line(10.0, 5.0).build();
        assert!((t.ground_y(5.0) - 2.5).abs() < 1e-5);
        assert_eq!(t.ground_y(-1.0), 0.0);
        assert_eq!(t.ground_y(11.0), 5.0);
    }

    #[test]
    fn curves_are_smooth_and_end_where_asked() {
        let t = TrackBuilder::new("t", 0.0, 0.0)
            .flat(5.0)
            .curve(8.0, 2.0, 0.5)
            .build();
        let end = t.ground.last().unwrap();
        assert!((end[0] - 13.0).abs() < 1e-4 && (end[1] - 2.0).abs() < 1e-4);
        // Starts flat: the first curve samples barely rise.
        assert!(t.ground_y(5.3) < 0.02);
    }

    #[test]
    fn json_roundtrip() {
        let t = Track::proving_grounds();
        assert_eq!(Track::from_json(&t.to_json()).unwrap(), t);
    }
}
