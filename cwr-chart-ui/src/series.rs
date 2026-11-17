//! Helpers for shaping time series before they are handed to D3.

/// Maximum number of points worth drawing on a ~900px-wide chart.
pub const MAX_CHART_POINTS: usize = 2000;

/// Evenly thin `points` down to roughly `max_points`, always keeping the first
/// and last element so the line still spans the full range.
pub fn downsample<T>(points: &[T], max_points: usize) -> Vec<&T> {
    if max_points < 2 || points.len() <= max_points {
        return points.iter().collect();
    }
    let last = points.len() - 1;
    let step = last as f64 / (max_points - 1) as f64;
    (0..max_points)
        .map(|i| &points[((i as f64 * step).round() as usize).min(last)])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_series_is_untouched() {
        let data = [1, 2, 3];
        assert_eq!(downsample(&data, 10), vec![&1, &2, &3]);
    }

    #[test]
    fn long_series_keeps_both_ends() {
        let data: Vec<usize> = (0..10_000).collect();
        let out = downsample(&data, 2000);
        assert_eq!(out.len(), 2000);
        assert_eq!(*out[0], 0);
        assert_eq!(**out.last().unwrap(), 9_999);
        assert!(out.windows(2).all(|w| w[0] < w[1]));
    }
}
