use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// A 2D vector or point in `f64`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DVec2 {
    /// x component.
    pub x: f64,
    /// y component.
    pub y: f64,
}

/// A 3D vector or point in `f64`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DVec3 {
    /// x component.
    pub x: f64,
    /// y component.
    pub y: f64,
    /// z component.
    pub z: f64,
}

impl DVec2 {
    /// (0, 0).
    pub const ZERO: Self = Self::new(0.0, 0.0);
    /// (1, 0).
    pub const X: Self = Self::new(1.0, 0.0);
    /// (0, 1).
    pub const Y: Self = Self::new(0.0, 1.0);

    /// Builds a vector from its components.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Dot product.
    pub fn dot(self, o: Self) -> f64 {
        self.x * o.x + self.y * o.y
    }

    /// The z component of the 3D cross product: positive when `o` is
    /// counter-clockwise of `self`. Not robust near zero; use `predicates::orient2d`
    /// when the sign decides topology.
    pub fn perp_dot(self, o: Self) -> f64 {
        self.x * o.y - self.y * o.x
    }

    /// Squared Euclidean length.
    pub fn length_squared(self) -> f64 {
        self.dot(self)
    }

    /// Euclidean length.
    pub fn length(self) -> f64 {
        self.length_squared().sqrt()
    }

    /// True if both components are finite.
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }

    /// The unit vector in this direction, or `None` for a zero or non-finite vector.
    /// Pre-scales by the largest component, so tiny but non-zero vectors (which would
    /// underflow `length_squared`) still have a direction.
    pub fn normalize(self) -> Option<Self> {
        let m = self.x.abs().max(self.y.abs());
        if !self.is_finite() || m == 0.0 {
            return None;
        }
        let v = self / m;
        Some(v / v.length())
    }
}

impl DVec3 {
    /// (0, 0, 0).
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);
    /// (1, 0, 0).
    pub const X: Self = Self::new(1.0, 0.0, 0.0);
    /// (0, 1, 0).
    pub const Y: Self = Self::new(0.0, 1.0, 0.0);
    /// (0, 0, 1).
    pub const Z: Self = Self::new(0.0, 0.0, 1.0);

    /// Builds a vector from its components.
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Dot product.
    pub fn dot(self, o: Self) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    /// Cross product (right-handed: `X.cross(Y) == Z`).
    pub fn cross(self, o: Self) -> Self {
        Self::new(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }

    /// Squared Euclidean length.
    pub fn length_squared(self) -> f64 {
        self.dot(self)
    }

    /// Euclidean length.
    pub fn length(self) -> f64 {
        self.length_squared().sqrt()
    }

    /// True if all components are finite.
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    /// The unit vector in this direction, or `None` for a zero or non-finite vector.
    /// See [`DVec2::normalize`] for why it pre-scales.
    pub fn normalize(self) -> Option<Self> {
        let m = self.x.abs().max(self.y.abs()).max(self.z.abs());
        if !self.is_finite() || m == 0.0 {
            return None;
        }
        let v = self / m;
        Some(v / v.length())
    }
}

macro_rules! vector_ops {
    ($t:ident { $($f:ident),+ }) => {
        impl Add for $t {
            type Output = Self;
            fn add(self, o: Self) -> Self { Self { $($f: self.$f + o.$f),+ } }
        }
        impl Sub for $t {
            type Output = Self;
            fn sub(self, o: Self) -> Self { Self { $($f: self.$f - o.$f),+ } }
        }
        impl Mul<f64> for $t {
            type Output = Self;
            fn mul(self, s: f64) -> Self { Self { $($f: self.$f * s),+ } }
        }
        impl Div<f64> for $t {
            type Output = Self;
            fn div(self, s: f64) -> Self { Self { $($f: self.$f / s),+ } }
        }
        impl Neg for $t {
            type Output = Self;
            fn neg(self) -> Self { Self { $($f: -self.$f),+ } }
        }
        impl AddAssign for $t {
            fn add_assign(&mut self, o: Self) { $(self.$f += o.$f;)+ }
        }
        impl SubAssign for $t {
            fn sub_assign(&mut self, o: Self) { $(self.$f -= o.$f;)+ }
        }
    };
}

vector_ops!(DVec2 { x, y });
vector_ops!(DVec3 { x, y, z });
