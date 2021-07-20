use std::fmt;

use serde::{Deserialize, Serialize};

use crate::elliptic::curves::traits::*;
use crate::BigInt;

use super::{
    error::{MismatchedPointOrder, PointFromBytesError, PointFromCoordsError},
    format::PointFormat,
    Generator, PointRef,
};
use crate::elliptic::curves::ZeroPointError;

/// Elliptic point of a [group order](super::Scalar::group_order), or a zero point
///
/// ## Guarantees
///
/// * On curve
///
///   Any instance of `Point<E>` is guaranteed to belong to curve `E`, i.e. its coordinates must
///   satisfy curve equations
/// * Point order equals to [group order](super::Scalar::group_order) (unless it's zero point)
///
///   I.e. denoting `q = group_order`, following predicate is always true:
///   `P = O ∨ qP = O ∧ forall 0 < s < q. sP ≠ O`
///
/// ## Security
///
/// Validate points if they come from untrusted source. Mistakenly used zero point might break security
/// of cryptoalgorithm. Use [ensure_nonzero](Point::ensure_nonzero) to validate them.
///
/// ```rust
/// # use curv::elliptic::curves::{Point, Curve, ZeroPointError};
/// # struct T;
/// fn process_input<E: Curve>(point: &Point<E>) -> Result<T, ZeroPointError> {
///     point.ensure_nonzero()?;
///     // ... process the point
///     # Ok(T)
/// }
/// ```
///
/// ## Arithmetics
///
/// You can add, subtract two points, or multiply point at scalar:
///
/// ```rust
/// # use curv::elliptic::curves::{Point, Scalar, Secp256k1};
/// fn expression(
///     a: Point<Secp256k1>,
///     b: Point<Secp256k1>,
///     c: Scalar<Secp256k1>,
/// ) -> Point<Secp256k1> {
///     a + b * c
/// }
/// ```
#[derive(Serialize, Deserialize)]
#[serde(try_from = "PointFormat<E>", into = "PointFormat<E>", bound = "")]
pub struct Point<E: Curve> {
    raw_point: E::Point,
}

impl<E: Curve> Point<E> {
    /// Ensures that `self` is not zero, returns `Err(_)` otherwise
    pub fn ensure_nonzero(&self) -> Result<(), ZeroPointError> {
        if self.is_zero() {
            Err(ZeroPointError::new())
        } else {
            Ok(())
        }
    }
    /// Curve generator
    ///
    /// Returns a structure holding a static reference on actual value (in most cases referenced
    /// value is fine). Use [`.to_point()`](Generator::to_point) if you need to take it by value.
    pub fn generator() -> Generator<E> {
        Generator::default()
    }

    /// Curve second generator
    ///
    /// We provide an alternative generator value and prove that it was picked randomly.
    ///
    /// Returns a structure holding a static reference on actual value (in most cases referenced
    /// value is fine). Use [`.to_point()`](PointRef::to_point) if you need to take it by value.
    pub fn base_point2() -> PointRef<'static, E> {
        let p = E::Point::base_point2();
        PointRef::from_raw(p).expect("base_point2 must have correct order")
    }

    /// Constructs zero point
    ///
    /// Zero point (or curve neutral element) is usually denoted as `O`. Its property: `forall A. A + O = A`.
    ///
    /// Weierstrass and Montgomery curves employ special "point at infinity" that represent a neutral
    /// element, such points don't have coordinates (i.e. [from_coords], [x_coord], [y_coord] return
    /// `None`). Edwards curves' neutral element has coordinates.
    ///
    /// [from_coords]: Self::from_coords
    /// [x_coord]: Self::x_coord
    /// [y_coord]: Self::y_coord
    pub fn zero() -> Self {
        // Safety: `self` can be constructed to hold a zero point
        unsafe { Self::from_raw_unchecked(E::Point::zero()) }
    }

    /// Checks whether point is zero
    pub fn is_zero(&self) -> bool {
        self.as_raw().is_zero()
    }

    /// Returns point coordinates
    ///
    /// Point might not have coordinates (specifically, "point at infinity" doesn't), in this case
    /// `None` is returned
    pub fn coords(&self) -> Option<PointCoords> {
        self.as_raw().coords()
    }

    /// Returns point x coordinate
    ///
    /// See [coords](Self::coords) method that retrieves both x and y at once.
    pub fn x_coord(&self) -> Option<BigInt> {
        self.as_raw().x_coord()
    }

    /// Returns point y coordinate
    ///
    /// See [coords](Self::coords) method that retrieves both x and y at once.
    pub fn y_coord(&self) -> Option<BigInt> {
        self.as_raw().y_coord()
    }

    /// Constructs a point from its coordinates, returns error if coordinates don't satisfy
    /// curve equation or if point has invalid order
    pub fn from_coords(x: &BigInt, y: &BigInt) -> Result<Self, PointFromCoordsError> {
        let raw_point = E::Point::from_coords(x, y)
            .map_err(|_: NotOnCurve| PointFromCoordsError::NotOnCurve)?;
        Self::from_raw(raw_point).map_err(PointFromCoordsError::InvalidPoint)
    }

    /// Creates [PointRef] that holds a reference on `self`
    pub fn as_point(&self) -> PointRef<E> {
        PointRef::from(self)
    }

    /// Tries to parse a point in (un)compressed form
    ///
    /// Whether it's in compressed or uncompressed form will be deduced from its length
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PointFromBytesError> {
        let p = E::Point::deserialize(bytes)
            .map_err(|_: DeserializationError| PointFromBytesError::DeserializationError)?;
        Self::from_raw(p).map_err(PointFromBytesError::InvalidPoint)
    }

    /// Serializes a point in (un)compressed form
    pub fn to_bytes(&self, compressed: bool) -> Vec<u8> {
        self.as_raw().serialize(compressed)
    }

    /// Constructs a `Point<E>` from low-level [ECPoint] implementor
    ///
    /// Returns error if point is not valid. Valid point is either a zero point, or a point of
    /// [group order].
    ///
    /// Typically, you don't need to use this constructor. See [generator](Point::generator),
    /// [base_point2](Point::base_point2), [from_coords](Self::from_coords), [from_bytes](Self::from_bytes)
    /// constructors, and `From<T>` and `TryFrom<T>` traits implemented for `Point<E>`.
    ///
    /// [ECPoint]: crate::elliptic::curves::ECPoint
    /// [group order]: crate::elliptic::curves::ECScalar::group_order
    pub fn from_raw(raw_point: E::Point) -> Result<Self, MismatchedPointOrder> {
        if raw_point.is_zero() || raw_point.check_point_order_equals_group_order() {
            Ok(Self { raw_point })
        } else {
            Err(MismatchedPointOrder::new())
        }
    }

    /// Constructs a `Point<E>` from low-level [ECPoint] implementor
    ///
    /// # Safety
    ///
    /// This function will not perform any checks against the point. You must guarantee that either
    /// point order is equal to curve [group order] or it's a zero point. To perform this check, you
    /// may use [ECPoint::check_point_order_equals_group_order][check_point_order_equals_group_order]
    /// and [ECPoint::is_zero][is_zero] methods.
    ///
    /// [ECPoint]: crate::elliptic::curves::ECPoint
    /// [group order]: crate::elliptic::curves::ECScalar::group_order
    /// [check_point_order_equals_group_order]: crate::elliptic::curves::ECPoint::check_point_order_equals_group_order
    /// [is_zero]: crate::elliptic::curves::ECPoint::is_zero
    pub unsafe fn from_raw_unchecked(raw_point: E::Point) -> Self {
        Self { raw_point }
    }

    /// Returns a reference to low-level point implementation
    ///
    /// Typically, you don't need to work with `ECPoint` trait directly. `Point<E>` wrapper
    /// provides convenient utilities around it: it implements arithmetic operators, (de)serialization
    /// traits, various getters (like [`.coords()`](Self::coords). If you believe that some functionality
    /// is missing, please [open an issue](https://github.com/ZenGo-X/curv).
    pub fn as_raw(&self) -> &E::Point {
        &self.raw_point
    }

    /// Converts a point into inner low-level point implementation
    ///
    /// Typically, you don't need to work with `ECPoint` trait directly. `Point<E>` wraps `ECPoint`
    /// and provides convenient utilities around it: it implements arithmetic operators, (de)serialization
    /// traits, various getters (like [`.coords()`](Self::coords)). If you believe that some functionality
    /// is missing, please [open an issue](https://github.com/ZenGo-X/curv).
    pub fn into_raw(self) -> E::Point {
        self.raw_point
    }
}

impl<E: Curve> PartialEq for Point<E> {
    fn eq(&self, other: &Self) -> bool {
        self.raw_point.eq(&other.raw_point)
    }
}

impl<'p, E: Curve> PartialEq<PointRef<'p, E>> for Point<E> {
    fn eq(&self, other: &PointRef<'p, E>) -> bool {
        self.as_raw().eq(other.as_raw())
    }
}

impl<E: Curve> PartialEq<Generator<E>> for Point<E> {
    fn eq(&self, other: &Generator<E>) -> bool {
        self.as_raw().eq(other.as_raw())
    }
}

impl<E: Curve> Clone for Point<E> {
    fn clone(&self) -> Self {
        // Safety: self is guaranteed to have correct order
        unsafe { Point::from_raw_unchecked(self.as_raw().clone()) }
    }
}

impl<E: Curve> fmt::Debug for Point<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.raw_point.fmt(f)
    }
}

impl<E: Curve> From<Generator<E>> for Point<E> {
    fn from(g: Generator<E>) -> Self {
        // Safety: curve generator order must be equal to group_order
        unsafe { Point::from_raw_unchecked(g.as_raw().clone()) }
    }
}

impl<'p, E: Curve> From<PointRef<'p, E>> for Point<E> {
    fn from(p: PointRef<E>) -> Self {
        // Safety: `PointRef` holds the same guarantees as `Point`
        unsafe { Point::from_raw_unchecked(p.as_raw().clone()) }
    }
}