//! Overlapping print pages over a sheet: pages are joined by overlapping, never trimmed.

use openplush_geom::DVec2;

use crate::{LayoutError, Sheet};

/// Unprinted border on every page edge, in millimetres.
const MARGIN_MM: f64 = 10.0;
/// How far each page repeats its right and bottom neighbour's edge, in millimetres.
const OVERLAP_MM: f64 = 15.0;
/// Pages along either side: one row letter per row, and the same cap across so the count stays bounded.
/// 26 A4 pages reach 4.5 m across and 6.8 m down, far past the 1000 mm largest plush.
const MAX_PAGES_PER_SIDE: u32 = 26;

/// A paper size, printed portrait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Paper {
    /// US Letter, 215.9 by 279.4 mm.
    Letter,
    /// ISO A4, 210 by 297 mm.
    A4,
}

impl Paper {
    /// Width and height of the printable area inside the margin, in millimetres.
    fn printable_mm(self) -> DVec2 {
        let (w, h) = match self {
            Self::Letter => (215.9, 279.4),
            Self::A4 => (210.0, 297.0),
        };
        DVec2::new(w - 2.0 * MARGIN_MM, h - 2.0 * MARGIN_MM)
    }
}

/// One printed page: its place in the grid and the part of the sheet it shows.
#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    row: u32,
    column: u32,
    window_min: DVec2,
    window_max: DVec2,
}

impl Page {
    /// The row, from 0 at the sheet's top edge; sheets are y-up, so that is its highest y.
    pub fn row(&self) -> u32 {
        self.row
    }

    /// The column, from 0 at the sheet's `x = 0` edge.
    pub fn column(&self) -> u32 {
        self.column
    }

    /// The lowest sheet corner the printable area shows, in millimetres; may lie past the sheet's edge.
    pub fn window_min(&self) -> DVec2 {
        self.window_min
    }

    /// The highest sheet corner the printable area shows, in millimetres; may lie past the sheet's edge.
    pub fn window_max(&self) -> DVec2 {
        self.window_max
    }
}

/// The pages that print one sheet at true scale.
#[derive(Debug, Clone, PartialEq)]
pub struct PageTiling {
    paper: Paper,
    rows: u32,
    columns: u32,
    pages: Vec<Page>,
}

impl PageTiling {
    /// The paper every page is printed on.
    pub fn paper(&self) -> Paper {
        self.paper
    }

    /// Pages down the sheet.
    pub fn rows(&self) -> u32 {
        self.rows
    }

    /// Pages across the sheet.
    pub fn columns(&self) -> u32 {
        self.columns
    }

    /// Every page, row by row, each row in column order.
    pub fn pages(&self) -> &[Page] {
        &self.pages
    }
}

/// Covers the whole sheet with the fewest pages from its top-left corner, each repeating 15 mm of its right and bottom neighbours.
pub fn tile(sheet: &Sheet, paper: Paper) -> Result<PageTiling, LayoutError> {
    let printable = paper.printable_mm();
    let too_many = || LayoutError::TooManyPages {
        width_mm: sheet.width_mm,
        length_mm: sheet.length_mm,
    };
    let columns = pages_along(sheet.width_mm, printable.x).ok_or_else(too_many)?;
    let rows = pages_along(sheet.length_mm, printable.y).ok_or_else(too_many)?;
    let step = DVec2::new(printable.x - OVERLAP_MM, printable.y - OVERLAP_MM);
    let pages = (0..rows)
        .flat_map(|row| (0..columns).map(move |column| (row, column)))
        .map(|(row, column)| {
            let top = sheet.length_mm - f64::from(row) * step.y;
            let window_min = DVec2::new(f64::from(column) * step.x, top - printable.y);
            Page {
                row,
                column,
                window_min,
                window_max: window_min + printable,
            }
        })
        .collect();
    Ok(PageTiling {
        paper,
        rows,
        columns,
        pages,
    })
}

/// The fewest pages that cover `extent` along one side, or `None` past the cap.
fn pages_along(extent: f64, printable: f64) -> Option<u32> {
    let step = printable - OVERLAP_MM;
    let mut n = 1;
    while f64::from(n - 1) * step + printable < extent {
        if n == MAX_PAGES_PER_SIDE {
            return None;
        }
        n += 1;
    }
    Some(n)
}
