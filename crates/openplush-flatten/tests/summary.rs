//! `summary::summarize`: one chart's distortion in numbers, against hand-computed soups.

use openplush_flatten::summary::{Summary, summarize};
use openplush_flatten::{FlattenError, StoppedBy};
use openplush_geom::{DVec2, DVec3};
use openplush_mesh::{FaceId, VertexId};
use openplush_seams::ChartId;

const CHART: ChartId = ChartId::new(2);

/// Separate right triangles, each laid flat with its legs scaled independently.
#[derive(Default)]
struct Soup {
    xyz: Vec<DVec3>,
    faces: Vec<(FaceId, [VertexId; 3])>,
    uv: Vec<DVec2>,
}

impl Soup {
    /// A right triangle with both legs `leg` long, flattened with its x leg scaled by `sx` and
    /// its y leg by `sy`: σ₁ = max(sx, sy), σ₂ = min(sx, sy), 3D area leg² / 2.
    fn push(mut self, leg: f64, sx: f64, sy: f64) -> Self {
        let first = u32::try_from(self.xyz.len()).unwrap_or(u32::MAX);
        let id = u32::try_from(self.faces.len()).unwrap_or(u32::MAX);
        self.xyz
            .extend([DVec3::ZERO, DVec3::X * leg, DVec3::Y * leg]);
        self.uv.extend([
            DVec2::ZERO,
            DVec2::new(leg * sx, 0.0),
            DVec2::new(0.0, leg * sy),
        ]);
        self.faces.push((
            FaceId::new(id),
            [first, first + 1, first + 2].map(VertexId::new),
        ));
        self
    }

    fn summarize(&self) -> Result<Summary, FlattenError> {
        summarize(CHART, &self.xyz, &self.faces, &self.uv, StoppedBy::Settled)
    }
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-12 * expected.abs().max(1.0),
        "{actual} is not {expected}"
    );
}

#[test]
fn an_isometric_chart_has_no_distortion() -> Result<(), FlattenError> {
    let s = Soup::default()
        .push(1.0, 1.0, 1.0)
        .push(3.0, 1.0, 1.0)
        .summarize()?;
    assert_eq!(s.most_stretched().0, FaceId::new(0));
    assert_close(s.most_stretched().1, 1.0);
    assert_close(s.most_compressed().1, 1.0);
    assert_close(s.stretch_excess(), 0.0);
    assert_close(s.compression_deficit(), 0.0);
    assert_close(s.tail_stretch(), 1.0);
    assert_close(s.tail_compression(), 1.0);
    assert_close(s.area_ratio(), 1.0);
    Ok(())
}

#[test]
fn means_are_weighted_by_3d_area() -> Result<(), FlattenError> {
    // Face 0: area 0.5, σ = (2, 1). Face 1: area 2, σ = (1, 0.5). Total area 2.5.
    let s = Soup::default()
        .push(1.0, 2.0, 1.0)
        .push(2.0, 1.0, 0.5)
        .summarize()?;
    assert_eq!(s.most_stretched(), (FaceId::new(0), 2.0));
    assert_eq!(s.most_compressed(), (FaceId::new(1), 0.5));
    // 0.5 · (2 − 1) / 2.5 and 2 · (1 − 0.5) / 2.5.
    assert_close(s.stretch_excess(), 0.2);
    assert_close(s.compression_deficit(), 0.4);
    // Flat area (0.5 · 2 + 2 · 0.5) over 3D area 2.5.
    assert_close(s.area_ratio(), 0.8);
    Ok(())
}

#[test]
fn the_tails_pass_over_a_worst_face_holding_under_five_percent_of_the_area()
-> Result<(), FlattenError> {
    // Face 0 holds 0.005 of 1.005 (under 5%); face 1 holds the rest.
    let s = Soup::default()
        .push(0.1, 4.0, 0.25)
        .push(1.0, 2.0, 0.5)
        .summarize()?;
    assert_eq!(s.most_stretched().0, FaceId::new(0));
    assert_close(s.most_stretched().1, 4.0);
    assert_eq!(s.most_compressed().0, FaceId::new(0));
    assert_close(s.most_compressed().1, 0.25);
    assert_close(s.tail_stretch(), 2.0);
    assert_close(s.tail_compression(), 0.5);
    Ok(())
}

#[test]
fn a_worst_face_holding_exactly_five_percent_of_the_area_is_the_tail() -> Result<(), FlattenError> {
    // Twenty equal faces: the first is 5% of the area.
    let soup = (1..20).fold(Soup::default().push(1.0, 3.0, 0.2), |s, _| {
        s.push(1.0, 2.0, 0.5)
    });
    let s = soup.summarize()?;
    assert_close(s.tail_stretch(), 3.0);
    assert_close(s.tail_compression(), 0.2);
    Ok(())
}

#[test]
fn the_tails_reach_past_a_face_holding_just_under_five_percent() -> Result<(), FlattenError> {
    // Face 0 holds 0.495 of 9.995, 4.95%, so the tail takes in face 1 as well.
    let first = Soup::default().push(0.99_f64.sqrt(), 3.0, 0.2);
    let soup = (1..20).fold(first, |s, i| {
        s.push(
            1.0,
            if i == 1 { 2.5 } else { 2.0 },
            if i == 1 { 0.4 } else { 0.5 },
        )
    });
    let s = soup.summarize()?;
    assert_close(s.tail_stretch(), 2.5);
    assert_close(s.tail_compression(), 0.4);
    Ok(())
}

#[test]
fn a_tie_for_the_worst_face_names_the_first_in_input_order() -> Result<(), FlattenError> {
    let s = Soup::default()
        .push(1.0, 1.0, 1.0)
        .push(1.0, 2.0, 0.5)
        .push(2.0, 2.0, 0.5)
        .summarize()?;
    assert_eq!(s.most_stretched(), (FaceId::new(1), 2.0));
    assert_eq!(s.most_compressed(), (FaceId::new(1), 0.5));
    Ok(())
}

#[test]
fn the_summary_does_not_depend_on_the_order_of_the_faces() -> Result<(), FlattenError> {
    let forward = Soup::default()
        .push(1.0, 2.0, 1.0)
        .push(2.0, 1.0, 0.5)
        .push(0.5, 1.5, 0.75)
        .summarize()?;
    let backward = Soup::default()
        .push(0.5, 1.5, 0.75)
        .push(2.0, 1.0, 0.5)
        .push(1.0, 2.0, 1.0)
        .summarize()?;
    assert_close(forward.stretch_excess(), backward.stretch_excess());
    assert_close(
        forward.compression_deficit(),
        backward.compression_deficit(),
    );
    assert_close(forward.tail_stretch(), backward.tail_stretch());
    assert_close(forward.tail_compression(), backward.tail_compression());
    assert_close(forward.area_ratio(), backward.area_ratio());
    Ok(())
}

#[test]
fn scaling_the_chart_leaves_the_summary_unchanged() -> Result<(), FlattenError> {
    let at = |leg: f64| {
        Soup::default()
            .push(leg, 2.0, 1.0)
            .push(2.0 * leg, 1.0, 0.5)
            .summarize()
    };
    for leg in [1e-70, 1e70] {
        let s = at(leg)?;
        assert_close(s.stretch_excess(), 0.2);
        assert_close(s.compression_deficit(), 0.4);
        assert_close(s.area_ratio(), 0.8);
    }
    Ok(())
}

#[test]
fn stretches_whose_weighted_sum_passes_f64_max_still_average() -> Result<(), FlattenError> {
    // σ₁ = 4·10³⁰⁷ on two faces of area 8: finite, but Σ A·σ₁ is not.
    let s = Soup::default()
        .push(4.0, 4e307, 1e-300)
        .push(4.0, 4e307, 1e-300)
        .summarize()?;
    assert_close(s.stretch_excess(), 4e307);
    assert_close(s.tail_stretch(), 4e307);
    assert_close(s.area_ratio(), 4e7);
    Ok(())
}

#[test]
fn a_shrunk_face_has_no_stretch_excess() -> Result<(), FlattenError> {
    let s = Soup::default()
        .push(1.0, 1.0, 1.0)
        .push(1.0, 0.5, 0.5)
        .summarize()?;
    assert_close(s.stretch_excess(), 0.0);
    assert_close(s.compression_deficit(), 0.25);
    Ok(())
}

#[test]
fn a_grown_face_has_no_compression_deficit() -> Result<(), FlattenError> {
    let s = Soup::default()
        .push(1.0, 1.0, 1.0)
        .push(1.0, 2.0, 2.0)
        .summarize()?;
    assert_close(s.compression_deficit(), 0.0);
    assert_close(s.stretch_excess(), 0.5);
    Ok(())
}

#[test]
fn a_chart_with_no_faces_is_refused() {
    assert_eq!(
        Soup::default().summarize(),
        Err(FlattenError::EmptyChart { chart: CHART })
    );
}

#[test]
fn a_flipped_face_is_refused_rather_than_summarized() {
    let soup = Soup::default().push(1.0, 1.0, 1.0).push(1.0, -1.0, 1.0);
    assert_eq!(
        soup.summarize(),
        Err(FlattenError::Flip {
            chart: CHART,
            faces: vec![FaceId::new(1)],
        })
    );
}

#[test]
fn the_summary_carries_the_stop_it_is_given() -> Result<(), FlattenError> {
    let soup = Soup::default().push(1.0, 1.0, 1.0);
    assert_eq!(soup.summarize()?.stopped_by(), StoppedBy::Settled);
    for stopped in [StoppedBy::IterationCap, StoppedBy::NoDescent] {
        let s = summarize(CHART, &soup.xyz, &soup.faces, &soup.uv, stopped)?;
        assert_eq!(s.stopped_by(), stopped);
    }
    Ok(())
}
