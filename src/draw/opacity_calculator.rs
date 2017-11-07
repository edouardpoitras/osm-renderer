use mapcss::styler::LineCap;
use std::cmp::Ordering;

pub struct OpacityCalculator {
    half_line_width: f64,
    dashes: Vec<DashSegment>,
    total_dash_len: f64,
    traveled_distance: f64,
}

pub struct OpacityData {
    pub opacity: f64,
    pub is_in_line: bool,
}

impl OpacityCalculator {
    pub fn new(line_width: f64, dashes: &Option<Vec<f64>>, line_cap: &Option<LineCap>) -> Self {
        let half_line_width = line_width / 2.0;
        let mut dash_segments = Vec::new();
        let mut len_before = 0.0;

        if let Some(ref dashes) = *dashes {
            compute_segments(half_line_width, dashes, line_cap, &mut dash_segments, &mut len_before);
        }

        Self {
            half_line_width,
            dashes: dash_segments,
            total_dash_len: len_before,
            traveled_distance: 0.0,
        }
    }

    pub fn calculate(&self, center_distance: f64, start_distance: f64) -> OpacityData {
        let sd = self.get_opacity_by_start_distance(start_distance);

        let half_line_width = sd.distance_in_cap.map(|cap_dist| {
            (self.half_line_width.powi(2) - cap_dist.powi(2)).sqrt()
        }).unwrap_or(self.half_line_width);

        let cd = get_opacity_by_center_distance(center_distance, half_line_width);
        OpacityData {
            opacity: sd.opacity.min(cd),
            is_in_line: cd > 0.0,
        }
    }

    pub fn add_traveled_distance(&mut self, distance: f64) {
        self.traveled_distance += distance;
    }

    fn get_opacity_by_start_distance(&self, start_distance: f64) -> StartDistanceOpacityData {
        if self.dashes.is_empty() {
            return StartDistanceOpacityData {
                opacity: 1.0,
                distance_in_cap: None,
            };
        }

        let dist_rem = (self.traveled_distance + start_distance) % self.total_dash_len;

        let best_dash_with_opacity = self.dashes.iter()
            .filter_map(|d| {
                get_opacity_by_segment(dist_rem, d).map(|op| (d, op))
            })
            .max_by(|x, y| x.1.partial_cmp(&y.1).unwrap_or(Ordering::Equal));

        StartDistanceOpacityData {
            opacity: best_dash_with_opacity.map(|x| x.1).unwrap_or(0.0),
            distance_in_cap: best_dash_with_opacity.map(|x| x.0).and_then(|d| {
                get_distance_in_cap(dist_rem, d)
            }),
        }
    }
}

struct StartDistanceOpacityData {
    opacity: f64,
    distance_in_cap: Option<f64>,
}

struct DashSegment {
    start_from: f64,
    start_to: f64,
    end_from: f64,
    end_to: f64,
    opacity_mul: f64,
    original_endpoints: Option<(f64, f64)>,
}

fn compute_segments(
    half_line_width: f64,
    dashes: &Vec<f64>,
    line_cap: &Option<LineCap>,
    segments: &mut Vec<DashSegment>,
    len_before: &mut f64
) {
    // Use the first dash twice to make sure we don't miss the very first cap.
    let dash_indexes = (0..dashes.len()).chain(0..1);

    for idx in dash_indexes {
        let dash = dashes[idx];
        let mut start = *len_before;

        if idx != 0 || segments.is_empty() {
            *len_before += dash;
        }

        if idx % 2 != 0 {
            continue;
        }

        let mut end = start + dash;

        let original_endpoints = match *line_cap {
            Some(LineCap::Round) => Some((start, end)),
            _ => None,
        };

        match *line_cap {
            Some(LineCap::Square) | Some(LineCap::Round) => {
                start -= half_line_width;
                end += half_line_width;
            },
            _ => {},
        }

        let midpoint = (start + end) / 2.0;

        segments.push(DashSegment {
            start_from: (start - 0.5).min(midpoint - 1.0),
            start_to: (start + 0.5).min(midpoint),
            end_from: (end - 0.5).max(midpoint),
            end_to: (end + 0.5).max(midpoint + 1.0),
            opacity_mul: (end - start).min(1.0),
            original_endpoints,
        })
    }
}

fn get_opacity_by_segment(dist: f64, segment: &DashSegment) -> Option<f64> {
    let base_opacity = if dist < segment.start_from {
        None
    } else if dist <= segment.start_to {
        Some((dist - segment.start_from) / (segment.start_to - segment.start_from))
    } else if dist < segment.end_from {
        Some(1.0)
    } else if dist <= segment.end_to {
        Some((segment.end_to - dist) / (segment.end_to - segment.end_from))
    } else {
        None
    };

    base_opacity.map(|op| segment.opacity_mul * op)
}

fn get_distance_in_cap(dist: f64, segment: &DashSegment) -> Option<f64> {
    segment.original_endpoints.and_then(|(a, b)| {
        if dist < a {
            Some(a - dist)
        } else if dist <= b {
            None
        } else {
            Some(dist - b)
        }
    })
}

fn get_opacity_by_center_distance(center_distance: f64, half_line_width: f64) -> f64 {
    let feather_from = (half_line_width - 0.5).max(0.0);
    let feather_to = (half_line_width + 0.5).max(1.0);
    let feather_dist = feather_to - feather_from;
    let opacity_mul = (2.0 * half_line_width).min(1.0);

    opacity_mul * (if center_distance < feather_from {
        1.0
    } else if center_distance < feather_to {
        (feather_to - center_distance) / feather_dist
    } else {
        0.0
    })
}
