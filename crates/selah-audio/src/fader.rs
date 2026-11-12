//! Fade curves and level conversion. Pure functions, safe to call from the
//! audio thread.

use selah_core::FadeCurve;

/// Levels at or below this are treated as silence.
pub const SILENCE_DB: f64 = -120.0;

/// Decibels to a linear amplitude multiplier.
pub fn db_to_linear(db: f64) -> f32 {
    if db <= SILENCE_DB {
        0.0
    } else {
        10f64.powf(db / 20.0) as f32
    }
}

/// A value between `from` and `to` at progress `p` (0 to 1) along `curve`.
///
/// `EqualPower` follows a sine (rising) or cosine (falling), so a rising and a
/// falling fade over the same time keep constant power when crossfaded.
pub fn ramp_value(from: f32, to: f32, p: f32, curve: FadeCurve) -> f32 {
    let p = p.clamp(0.0, 1.0);
    match curve {
        FadeCurve::Linear => from + (to - from) * p,
        FadeCurve::SCurve => from + (to - from) * (p * p * (3.0 - 2.0 * p)),
        FadeCurve::EqualPower => {
            let quarter = p * std::f32::consts::FRAC_PI_2;
            if to >= from {
                from + (to - from) * quarter.sin()
            } else {
                to + (from - to) * quarter.cos()
            }
        }
    }
}

/// A gain that moves from one value to another over a number of frames.
#[derive(Debug, Clone, Copy)]
pub struct Ramp {
    from: f32,
    to: f32,
    len: u32,
    pos: u32,
    curve: FadeCurve,
}

impl Ramp {
    /// A ramp that is already at `value`.
    pub fn steady(value: f32) -> Self {
        Self {
            from: value,
            to: value,
            len: 0,
            pos: 0,
            curve: FadeCurve::Linear,
        }
    }

    pub fn current(&self) -> f32 {
        if self.pos >= self.len {
            self.to
        } else {
            ramp_value(
                self.from,
                self.to,
                self.pos as f32 / self.len as f32,
                self.curve,
            )
        }
    }

    pub fn is_done(&self) -> bool {
        self.pos >= self.len
    }

    pub fn target(&self) -> f32 {
        self.to
    }

    /// Starts moving from wherever the gain is now. Zero frames jumps at once.
    pub fn retarget(&mut self, to: f32, frames: u32, curve: FadeCurve) {
        self.from = self.current();
        self.to = to;
        self.len = frames;
        self.pos = 0;
        self.curve = curve;
    }

    /// Moves on by one frame.
    pub fn step(&mut self) {
        if self.pos < self.len {
            self.pos += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decibels_convert_and_silence_is_exactly_zero() {
        assert!((db_to_linear(0.0) - 1.0).abs() < 1e-6);
        assert!((db_to_linear(-6.0206) - 0.5).abs() < 1e-3);
        assert_eq!(db_to_linear(-120.0), 0.0);
        assert_eq!(db_to_linear(-200.0), 0.0);
    }

    #[test]
    fn every_curve_starts_and_ends_exactly() {
        for curve in [FadeCurve::Linear, FadeCurve::EqualPower, FadeCurve::SCurve] {
            for (a, b) in [(0.0, 1.0), (1.0, 0.0), (0.3, 0.8)] {
                assert!((ramp_value(a, b, 0.0, curve) - a).abs() < 1e-6, "{curve:?}");
                assert!((ramp_value(a, b, 1.0, curve) - b).abs() < 1e-6, "{curve:?}");
            }
        }
    }

    #[test]
    fn an_equal_power_crossfade_keeps_constant_power() {
        for i in 0..=20 {
            let p = i as f32 / 20.0;
            let rising = ramp_value(0.0, 1.0, p, FadeCurve::EqualPower);
            let falling = ramp_value(1.0, 0.0, p, FadeCurve::EqualPower);
            assert!((rising * rising + falling * falling - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn linear_is_halfway_at_the_middle() {
        assert!((ramp_value(0.0, 1.0, 0.5, FadeCurve::Linear) - 0.5).abs() < 1e-6);
        assert!((ramp_value(0.0, 1.0, 0.5, FadeCurve::SCurve) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn a_ramp_takes_exactly_its_length_in_frames() {
        let mut ramp = Ramp::steady(0.0);
        ramp.retarget(1.0, 4, FadeCurve::Linear);
        let mut seen = Vec::new();
        for _ in 0..5 {
            seen.push(ramp.current());
            ramp.step();
        }
        assert_eq!(seen, [0.0, 0.25, 0.5, 0.75, 1.0]);
        assert!(ramp.is_done());
    }

    #[test]
    fn retargeting_starts_from_the_current_value() {
        let mut ramp = Ramp::steady(1.0);
        ramp.retarget(0.0, 10, FadeCurve::Linear);
        for _ in 0..5 {
            ramp.step();
        }
        ramp.retarget(1.0, 10, FadeCurve::Linear);
        assert!((ramp.current() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn zero_frames_jumps_at_once() {
        let mut ramp = Ramp::steady(1.0);
        ramp.retarget(0.25, 0, FadeCurve::EqualPower);
        assert_eq!(ramp.current(), 0.25);
    }
}
