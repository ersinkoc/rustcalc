//! Plot viewport, tick spacing and curve sampling. Independent of the GUI so
//! pan/zoom arithmetic and discontinuity detection can be tested directly.

/// Smallest and largest half-span; beyond these, f32 screen mapping and
/// decimal conversion of `x` lose meaning.
pub const MIN_SPAN: f64 = 1e-4;
pub const MAX_SPAN: f64 = 1e6;
const MAX_CENTER: f64 = 1e9;

/// Both axes share one numeric half-span around an independent center.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub center_x: f64,
    pub center_y: f64,
    pub span: f64,
}
impl Default for Viewport {
    fn default() -> Self {
        Self {
            center_x: 0.0,
            center_y: 0.0,
            span: 10.0,
        }
    }
}
impl Viewport {
    pub fn x_range(&self) -> (f64, f64) {
        (self.center_x - self.span, self.center_x + self.span)
    }
    pub fn y_range(&self) -> (f64, f64) {
        (self.center_y - self.span, self.center_y + self.span)
    }
    /// Move by a fraction of the full visible width/height (positive = right/up).
    pub fn pan(&mut self, dx: f64, dy: f64) {
        if dx.is_finite() && dy.is_finite() {
            self.center_x += dx * 2.0 * self.span;
            self.center_y += dy * 2.0 * self.span;
            self.clamp();
        }
    }
    /// Scale the span by `factor` (< 1 zooms in), keeping the anchor point fixed.
    pub fn zoom_at(&mut self, factor: f64, anchor_x: f64, anchor_y: f64) {
        if !(factor.is_finite() && factor > 0.0 && anchor_x.is_finite() && anchor_y.is_finite()) {
            return;
        }
        let span = (self.span * factor).clamp(MIN_SPAN, MAX_SPAN);
        let applied = span / self.span;
        self.center_x = anchor_x + (self.center_x - anchor_x) * applied;
        self.center_y = anchor_y + (self.center_y - anchor_y) * applied;
        self.span = span;
        self.clamp();
    }
    pub fn clamp(&mut self) {
        if !self.span.is_finite() {
            self.span = Self::default().span;
        }
        self.span = self.span.clamp(MIN_SPAN, MAX_SPAN);
        for c in [&mut self.center_x, &mut self.center_y] {
            *c = if c.is_finite() {
                c.clamp(-MAX_CENTER, MAX_CENTER)
            } else {
                0.0
            };
        }
    }
}

/// A 1-2-5 step giving roughly `target` intervals across `width`.
pub fn tick_step(width: f64, target: usize) -> f64 {
    let raw = width / target.max(1) as f64;
    if !(raw.is_finite() && raw > 0.0) {
        return 1.0;
    }
    let magnitude = 10f64.powf(raw.log10().floor());
    let normalized = raw / magnitude;
    let nice = if normalized < 1.5 {
        1.0
    } else if normalized < 3.5 {
        2.0
    } else if normalized < 7.5 {
        5.0
    } else {
        10.0
    };
    nice * magnitude
}

/// Multiples of `step` within `[min, max]`, with values near zero snapped to zero.
pub fn ticks(min: f64, max: f64, step: f64) -> Vec<f64> {
    if !(step > 0.0 && min.is_finite() && max.is_finite() && min <= max) {
        return Vec::new();
    }
    let first = (min / step).ceil() as i64;
    let last = (max / step).floor() as i64;
    if last - first > 1000 {
        return Vec::new();
    }
    (first..=last)
        .map(|i| {
            let v = i as f64 * step;
            if v.abs() < step * 1e-9 { 0.0 } else { v }
        })
        .collect()
}

/// A tick label with only as many decimals as the step needs.
pub fn tick_label(value: f64, step: f64) -> String {
    if value != 0.0 && (value.abs() >= 1e6 || step < 1e-4) {
        return format!("{value:.2e}");
    }
    let decimals = (-step.log10().floor()).clamp(0.0, 6.0) as usize;
    format!("{value:.decimals$}")
}

/// A readable value for the hover readout.
pub fn readout(value: f64) -> String {
    if value != 0.0 && !(1e-4..1e7).contains(&value.abs()) {
        format!("{value:.6e}")
    } else {
        let text = format!("{value:.6}");
        let text = text.trim_end_matches('0').trim_end_matches('.');
        if text == "-0" {
            "0".into()
        } else {
            text.into()
        }
    }
}

/// Connected runs of points; a gap separates undefined values and discontinuities.
pub type Segments = Vec<Vec<(f64, f64)>>;

/// Sample `f` across the viewport. Large jumps between neighbours are bisected:
/// a continuous steep curve shrinks with each halving, while a pole or jump
/// keeps its size and splits the curve instead of drawing a false vertical line.
pub fn sample(f: impl Fn(f64) -> Option<f64>, view: &Viewport, samples: usize) -> Segments {
    let samples = samples.clamp(2, 4000);
    let (x0, x1) = view.x_range();
    let threshold = view.span * 0.5;
    let mut segments: Segments = Vec::new();
    let mut current: Vec<(f64, f64)> = Vec::new();
    let mut last: Option<(f64, f64)> = None;
    for i in 0..=samples {
        let x = x0 + (x1 - x0) * i as f64 / samples as f64;
        let point = f(x).filter(|y| y.is_finite()).map(|y| (x, y));
        match (last, point) {
            (Some(a), Some(b)) => {
                if (b.1 - a.1).abs() > threshold && breaks(&f, a, b, threshold) {
                    segments.push(std::mem::take(&mut current));
                }
                current.push(b);
            }
            (None, Some(b)) => current.push(b),
            (_, None) => {
                if !current.is_empty() {
                    segments.push(std::mem::take(&mut current));
                }
            }
        }
        last = point;
    }
    if !current.is_empty() {
        segments.push(current);
    }
    segments.retain(|s| !s.is_empty());
    segments
}

fn breaks(
    f: &impl Fn(f64) -> Option<f64>,
    mut a: (f64, f64),
    mut b: (f64, f64),
    threshold: f64,
) -> bool {
    let original = (b.1 - a.1).abs();
    for _ in 0..12 {
        let x = (a.0 + b.0) / 2.0;
        let Some(y) = f(x).filter(|y| y.is_finite()) else {
            return true;
        };
        // Follow the half that still carries most of the jump.
        if (y - a.1).abs() >= (b.1 - y).abs() {
            b = (x, y);
        } else {
            a = (x, y);
        }
    }
    let remaining = (b.1 - a.1).abs();
    remaining > threshold.min(original) * 0.25
}

/// Roots of `f` in `[x0, x1]`: exact zeros and sign changes between samples,
/// refined by bisection. A sign change across a pole (`1/x`, `tan`) or a jump
/// is rejected because the values beside it stay large instead of vanishing.
/// Roots that only touch zero without crossing it are not reported.
pub fn roots(f: impl Fn(f64) -> Option<f64>, (x0, x1): (f64, f64), samples: usize) -> Vec<f64> {
    let samples = samples.clamp(2, 4000);
    let width = x1 - x0;
    if !(width.is_finite() && width > 0.0) {
        return Vec::new();
    }
    let mut found: Vec<f64> = Vec::new();
    let mut previous: Option<(f64, f64)> = None;
    for i in 0..=samples {
        let x = x0 + width * i as f64 / samples as f64;
        let current = f(x).filter(|y| y.is_finite()).map(|y| (x, y));
        let root = match (previous, current) {
            (_, Some((x, 0.0))) => Some(x),
            (Some(a), Some(b)) if a.1 != 0.0 && a.1.signum() != b.1.signum() => bisect(&f, a, b),
            _ => None,
        };
        if let Some(mut root) = root {
            if root.abs() < width * 1e-12 {
                root = 0.0;
            }
            if found
                .last()
                .is_none_or(|last| (root - last).abs() > width * 1e-9)
            {
                found.push(root);
            }
        }
        previous = current;
    }
    found
}

fn bisect(f: &impl Fn(f64) -> Option<f64>, mut a: (f64, f64), mut b: (f64, f64)) -> Option<f64> {
    let scale = a.1.abs().max(b.1.abs()).max(1.0);
    for _ in 0..200 {
        let x = (a.0 + b.0) / 2.0;
        if x == a.0 || x == b.0 {
            break;
        }
        let y = f(x).filter(|y| y.is_finite())?;
        if y == 0.0 {
            return Some(x);
        }
        if y.signum() == a.1.signum() {
            a = (x, y);
        } else {
            b = (x, y);
        }
    }
    (a.1.abs().min(b.1.abs()) <= scale * 1e-6).then_some((a.0 + b.0) / 2.0)
}

/// Clip a segment to the rectangle, returning the visible part (Liang–Barsky).
/// Done in f64 so far-off points never reach f32 screen coordinates.
pub fn clip(
    a: (f64, f64),
    b: (f64, f64),
    (x_min, x_max): (f64, f64),
    (y_min, y_max): (f64, f64),
) -> Option<((f64, f64), (f64, f64))> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let mut t0: f64 = 0.0;
    let mut t1: f64 = 1.0;
    for (p, q) in [
        (-dx, a.0 - x_min),
        (dx, x_max - a.0),
        (-dy, a.1 - y_min),
        (dy, y_max - a.1),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
            if t0 > t1 {
                return None;
            }
        }
    }
    Some((
        (a.0 + t0 * dx, a.1 + t0 * dy),
        (a.0 + t1 * dx, a.1 + t1 * dy),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_keeps_the_anchor_fixed_and_respects_limits() {
        let mut view = Viewport::default();
        view.zoom_at(0.5, 4.0, 2.0);
        assert_eq!(view.span, 5.0);
        assert_eq!((view.center_x, view.center_y), (2.0, 1.0));
        view.zoom_at(1e-12, 0.0, 0.0);
        assert_eq!(view.span, MIN_SPAN);
        view.zoom_at(1e30, 0.0, 0.0);
        assert_eq!(view.span, MAX_SPAN);
        let before = view;
        view.zoom_at(f64::NAN, 0.0, 0.0);
        view.zoom_at(0.0, 0.0, 0.0);
        assert_eq!(view, before);
    }

    #[test]
    fn pan_moves_by_visible_fractions_and_stays_bounded() {
        let mut view = Viewport::default();
        view.pan(0.25, -0.5);
        assert_eq!((view.center_x, view.center_y), (5.0, -10.0));
        view.pan(f64::INFINITY, 0.0);
        assert_eq!((view.center_x, view.center_y), (5.0, -10.0));
        view.span = MAX_SPAN;
        view.pan(1e6, 0.0);
        assert_eq!(view.center_x, MAX_CENTER);
    }

    #[test]
    fn ticks_use_one_two_five_steps_and_clean_labels() {
        assert_eq!(tick_step(20.0, 10), 2.0);
        assert_eq!(tick_step(1.0, 10), 0.1);
        assert_eq!(tick_step(70.0, 10), 5.0);
        assert_eq!(tick_step(0.0, 10), 1.0);
        assert_eq!(ticks(-1.0, 1.0, 0.5), vec![-1.0, -0.5, 0.0, 0.5, 1.0]);
        assert!(ticks(0.0, 1.0, 1e-9).is_empty());
        let near_zero = ticks(-0.3, 0.3, 0.1);
        assert!(near_zero.contains(&0.0));
        assert_eq!(tick_label(0.30000000000000004, 0.1), "0.3");
        assert_eq!(tick_label(20.0, 5.0), "20");
        assert_eq!(tick_label(2_000_000.0, 1e6), "2.00e6");
        assert_eq!(readout(0.5), "0.5");
        assert_eq!(readout(-0.0000001), "-1.000000e-7");
        assert_eq!(readout(-0.0), "0");
    }

    #[test]
    fn poles_and_jumps_split_curves_but_steep_slopes_do_not() {
        let view = Viewport::default();
        let reciprocal = sample(|x| (x != 0.0).then(|| 1.0 / x), &view, 400);
        assert_eq!(reciprocal.len(), 2);
        assert!(reciprocal[0].iter().all(|p| p.0 < 0.0));
        assert!(reciprocal[1].iter().all(|p| p.0 > 0.0));

        let tan = sample(|x: f64| Some(x.tan()), &view, 600);
        // tan has poles at ±π/2, ±3π/2, ±5π/2 inside ±10: six poles, seven pieces.
        assert_eq!(tan.len(), 7);

        let step = sample(|x: f64| Some(if x < 1.0 { 0.0 } else { 8.0 }), &view, 400);
        assert_eq!(step.len(), 2);

        let steep = sample(|x: f64| Some(x.powi(3)), &view, 400);
        assert_eq!(steep.len(), 1);
        let line = sample(|x| Some(50.0 * x), &view, 50);
        assert_eq!(line.len(), 1);
    }

    #[test]
    fn undefined_regions_end_segments() {
        let view = Viewport::default();
        let sqrt = sample(|x: f64| (x >= 0.0).then(|| x.sqrt()), &view, 400);
        assert_eq!(sqrt.len(), 1);
        assert!(sqrt[0][0].0 >= 0.0);
        assert!(sample(|_| None, &view, 100).is_empty());
        assert!(sample(|_| Some(f64::NAN), &view, 100).is_empty());
    }

    #[test]
    fn roots_are_refined_and_poles_or_jumps_are_not_roots() {
        let close = |found: &[f64], expected: &[f64]| {
            assert_eq!(found.len(), expected.len(), "{found:?}");
            for (a, b) in found.iter().zip(expected) {
                assert!((a - b).abs() < 1e-9, "{found:?}");
            }
        };
        let pi = std::f64::consts::PI;
        close(
            &roots(|x: f64| Some(x.sin()), (-10.0, 10.0), 333),
            &[-3.0 * pi, -2.0 * pi, -pi, 0.0, pi, 2.0 * pi, 3.0 * pi],
        );
        close(
            &roots(|x| Some(x * x - 2.0), (-10.0, 10.0), 97),
            &[-2f64.sqrt(), 2f64.sqrt()],
        );
        assert!(roots(|x: f64| (x != 0.0).then(|| 1.0 / x), (-10.0, 10.0), 400).is_empty());
        let tan = roots(|x: f64| Some(x.tan()), (-3.0, 3.0), 600);
        // ±π lie outside ±3; the poles at ±π/2 are rejected.
        close(&tan, &[0.0]);
        assert!(
            roots(
                |x: f64| Some(if x < 0.3 { -1.0 } else { 1.0 }),
                (-1.0, 1.0),
                100
            )
            .is_empty()
        );
        assert!(roots(|x: f64| Some(x * x + 1.0), (-10.0, 10.0), 100).is_empty());
        assert!(roots(|_| None, (-1.0, 1.0), 100).is_empty());
        assert!(roots(Some, (1.0, 1.0), 100).is_empty());
        // Exact zero at a sample point is reported once.
        assert_eq!(roots(Some, (-1.0, 1.0), 10), vec![0.0]);
    }

    #[test]
    fn clipping_keeps_the_true_slope_inside_the_view() {
        let r = (-1.0, 1.0);
        assert_eq!(
            clip((0.0, 0.0), (0.5, 0.5), r, r),
            Some(((0.0, 0.0), (0.5, 0.5)))
        );
        let (a, b) = clip((0.0, 0.0), (1.0, 1e12), r, r).unwrap();
        assert_eq!(a, (0.0, 0.0));
        assert!((b.1 - 1.0).abs() < 1e-9 && b.0 < 1e-9);
        assert_eq!(clip((2.0, 2.0), (3.0, 3.0), r, r), None);
        assert_eq!(
            clip((-2.0, 0.0), (2.0, 0.0), r, r),
            Some(((-1.0, 0.0), (1.0, 0.0)))
        );
        assert_eq!(clip((5.0, -5.0), (5.0, 5.0), r, r), None);
    }
}
