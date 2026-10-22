//! A model's transform stack and the file's axis system, from `Properties70`.

use crate::affine::{IDENTITY, Mat, mul};

use super::tree::{Node, Value, integer, number, text};

/// A `Properties70` node's `P` entries, looked up by name; an absent node has none, so
/// every property takes its FBX SDK default.
#[derive(Clone, Copy)]
pub(super) struct Properties<'a>(Option<&'a Node>);

impl<'a> Properties<'a> {
    /// The `Properties70` child of `owner`.
    pub(super) fn of(owner: Option<&'a Node>) -> Self {
        Self(owner.and_then(|o| o.child("Properties70")))
    }

    /// The values of property `name`: a `P` node's attributes after its name, type,
    /// label and flags.
    fn values(self, name: &str) -> Option<&'a [Value]> {
        let p = self
            .0?
            .children_named("P")
            .find(|p| text(p.attr(0)) == Some(name))?;
        Some(p.attrs.get(4..).unwrap_or_default())
    }

    fn vector(self, name: &str, default: [f64; 3]) -> Result<[f64; 3], String> {
        let Some(values) = self.values(name) else {
            return Ok(default);
        };
        let at = |i: usize| number(values.get(i));
        match (at(0), at(1), at(2)) {
            (Some(x), Some(y), Some(z)) => Ok([x, y, z]),
            _ => Err(format!("property {name} is not three numbers")),
        }
    }

    fn int(self, name: &str, default: i64) -> Result<i64, String> {
        match self.values(name) {
            None => Ok(default),
            Some(values) => {
                integer(values.first()).ok_or_else(|| format!("property {name} is not an integer"))
            }
        }
    }
}

/// The model's own transform: T Roff Rp Rpre R Rpost⁻¹ Rp⁻¹ Soff Sp S Sp⁻¹, the FBX SDK's
/// order, with pre- and post-rotation in XYZ order and R in the model's `RotationOrder`.
pub(super) fn local(props: Properties<'_>) -> Result<Mat, String> {
    let order = match props.int("RotationOrder", 0)? {
        // 6, spheric XYZ, has no single Euler order to read it in.
        o @ 0..=5 => ORDERS[o as usize],
        o => return Err(format!("property RotationOrder is {o}, not an Euler order")),
    };
    let zero = [0.0; 3];
    let rp = props.vector("RotationPivot", zero)?;
    let sp = props.vector("ScalingPivot", zero)?;
    let post = euler(ORDERS[0], props.vector("PostRotation", zero)?);
    let stack = [
        translate(props.vector("Lcl Translation", zero)?),
        translate(props.vector("RotationOffset", zero)?),
        translate(rp),
        euler(ORDERS[0], props.vector("PreRotation", zero)?),
        euler(order, props.vector("Lcl Rotation", zero)?),
        transpose(&post),
        translate(rp.map(|c| -c)),
        translate(props.vector("ScalingOffset", zero)?),
        translate(sp),
        scale(props.vector("Lcl Scaling", [1.0; 3])?),
        translate(sp.map(|c| -c)),
    ];
    Ok(stack.iter().fold(IDENTITY, |m, s| mul(&m, s)))
}

/// The geometric transform, which moves the model's own geometry and no child's.
pub(super) fn geometric(props: Properties<'_>) -> Result<Mat, String> {
    let t = translate(props.vector("GeometricTranslation", [0.0; 3])?);
    let r = euler(ORDERS[0], props.vector("GeometricRotation", [0.0; 3])?);
    let s = scale(props.vector("GeometricScaling", [1.0; 3])?);
    Ok(mul(&mul(&t, &r), &s))
}

/// The file's axis system turned to Y up, +Z front and +X right:
/// (CoordSign·p[Coord], UpSign·p[Up], FrontSign·p[Front]). Absent settings are those axes.
pub(super) fn axes(settings: Properties<'_>) -> Result<Mat, String> {
    let mut m = [[0.0; 4]; 4];
    m[3][3] = 1.0;
    let mut used = [false; 3];
    for (row, (name, default)) in [("Coord", 0), ("Up", 1), ("Front", 2)]
        .into_iter()
        .enumerate()
    {
        let axis = settings.int(&format!("{name}Axis"), default)?;
        let sign = settings.int(&format!("{name}AxisSign"), 1)?;
        let Some(a) = usize::try_from(axis).ok().filter(|&a| a < 3) else {
            return Err(format!("{name}Axis is {axis}, not 0, 1 or 2"));
        };
        if std::mem::replace(&mut used[a], true) {
            return Err(format!("{name}Axis is {axis}, an axis already used"));
        }
        if sign != 1 && sign != -1 {
            return Err(format!("{name}AxisSign is {sign}, not 1 or -1"));
        }
        m[a][row] = sign as f64;
    }
    Ok(m)
}

#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
    Z,
}

/// `RotationOrder` 0 to 5: the axes in the order they turn, the first applied first.
const ORDERS: [[Axis; 3]; 6] = {
    use Axis::{X, Y, Z};
    [
        [X, Y, Z],
        [X, Z, Y],
        [Y, Z, X],
        [Y, X, Z],
        [Z, X, Y],
        [Z, Y, X],
    ]
};

/// Turns by `degrees` about x, y and z, applied in `order`.
fn euler(order: [Axis; 3], degrees: [f64; 3]) -> Mat {
    order.iter().fold(IDENTITY, |m, &axis| {
        let angle = match axis {
            Axis::X => degrees[0],
            Axis::Y => degrees[1],
            Axis::Z => degrees[2],
        };
        mul(&rotation(axis, angle), &m)
    })
}

/// A right-handed turn about `axis`. `libm`, not `std`, so native and wasm32 agree to the bit.
fn rotation(axis: Axis, degrees: f64) -> Mat {
    let (s, c) = (
        libm::sin(degrees.to_radians()),
        libm::cos(degrees.to_radians()),
    );
    let (i, j) = match axis {
        Axis::X => (1, 2),
        Axis::Y => (2, 0),
        Axis::Z => (0, 1),
    };
    let mut m = IDENTITY;
    m[i][i] = c;
    m[i][j] = s;
    m[j][i] = -s;
    m[j][j] = c;
    m
}

fn translate([x, y, z]: [f64; 3]) -> Mat {
    let mut m = IDENTITY;
    m[3] = [x, y, z, 1.0];
    m
}

fn scale([x, y, z]: [f64; 3]) -> Mat {
    let mut m = IDENTITY;
    m[0][0] = x;
    m[1][1] = y;
    m[2][2] = z;
    m
}

/// The inverse of a rotation.
fn transpose(m: &Mat) -> Mat {
    let mut t = IDENTITY;
    for (i, column) in t.iter_mut().enumerate().take(3) {
        for (j, cell) in column.iter_mut().enumerate().take(3) {
            *cell = m[j][i];
        }
    }
    t
}
