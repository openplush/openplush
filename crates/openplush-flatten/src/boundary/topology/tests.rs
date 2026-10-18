use super::boundary_loop;
use crate::boundary::BoundaryError;
use crate::boundary::fixtures::{Faces, faces, grid, torus};

#[test]
fn the_loop_starts_at_its_lowest_vertex_and_follows_the_faces_winding() -> Result<(), BoundaryError>
{
    assert_eq!(boundary_loop(&grid(2, 2))?, [0, 1, 2, 5, 8, 7, 6, 3]);
    assert_eq!(boundary_loop(&faces(&[[2, 0, 1]]))?, [0, 1, 2]);
    assert_eq!(boundary_loop(&faces(&[[0, 2, 1]]))?, [0, 2, 1]);
    Ok(())
}

#[test]
fn a_surface_that_is_not_a_disk_is_refused() {
    let disk = grid(3, 3);
    let closed = torus(16);
    // Connected with one boundary loop, but the handle leaves Euler characteristic -1.
    let handle: Faces = closed.iter().skip(2).copied().collect();
    // A torus adds no Euler characteristic, so only the connectivity check catches it.
    let disk_and_closed: Faces = disk.iter().chain(&closed).copied().collect();
    let annulus: Faces = disk
        .iter()
        .copied()
        .filter(|(f, _)| f.index() / 2 != 4)
        .collect();
    let two_disks: Faces = faces(&[[0, 1, 5], [10, 11, 15]]);
    // Three quads in a ring, glued with a half twist: one boundary loop, but no consistent winding.
    let mobius = faces(&[
        [3, 4, 1],
        [3, 1, 0],
        [4, 5, 2],
        [4, 2, 1],
        [5, 0, 3],
        [5, 3, 2],
    ]);
    // Two triangles sharing a corner: Euler characteristic 1 and connected, but the walk
    // out of the shared corner either never returns or returns after one triangle.
    let bowtie = faces(&[[0, 1, 5], [5, 6, 10]]);
    let bowtie_closing_early = faces(&[[5, 6, 7], [5, 8, 9]]);
    let same_way_twice = faces(&[[0, 1, 5], [0, 1, 4]]);
    for (name, chart) in [
        ("no faces", Vec::new()),
        ("closed", closed),
        ("handle", handle),
        ("disk and closed", disk_and_closed),
        ("annulus", annulus),
        ("two disks", two_disks),
        ("mobius", mobius),
        ("bowtie", bowtie),
        ("bowtie closing early", bowtie_closing_early),
        ("same way twice", same_way_twice),
    ] {
        assert_eq!(
            boundary_loop(&chart),
            Err(BoundaryError::NotADisk),
            "{name}"
        );
    }
}
