#![allow(non_snake_case)]

// Starknet elliptic curve utility functions (se: https://docs.starkware.co/starkex/crypto/stark-curve.html).

use core::fmt;
use super::traits::{ECPoint, ECScalar};
use crate::arithmetic::traits::*;
use crate::BigInt;
use crate::ErrorKey;
use zeroize::Zeroize;
use std::ops::{Add, Mul, Sub};
use std::ptr;
use std::sync::atomic;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{Error, MapAccess, SeqAccess, Visitor};
use serde::ser::SerializeStruct;
pub use starknet_curve::{AffinePoint, ProjectivePoint, curve_params::*};
pub use starknet_ff::FieldElement;

pub type SK = FieldElement;
pub type PK = AffinePoint;

#[derive(Clone, Debug, Copy)]
pub struct StarknetCurveScalar {
    purpose: &'static str,
    fe: SK,
}

#[derive(Clone, Debug, Copy)]
pub struct StarknetCurvePoint {
    purpose: &'static str,
    ge: Option<PK>,
}
pub type GE = StarknetCurvePoint;
pub type FE = StarknetCurveScalar;

impl StarknetCurvePoint {
    pub fn random_point() -> StarknetCurvePoint {
        let random_scalar: StarknetCurveScalar = StarknetCurveScalar::new_random();
        let base_point = StarknetCurvePoint::generator();
        let pk = base_point.scalar_mul(&random_scalar.get_element());
        StarknetCurvePoint {
            purpose: "random_point",
            ge: pk.get_element(),
        }
    }
}

impl Zeroize for StarknetCurveScalar {
    fn zeroize(&mut self) {
        unsafe { ptr::write_volatile(self, FE::zero()) };
        atomic::fence(atomic::Ordering::SeqCst);
        atomic::compiler_fence(atomic::Ordering::SeqCst);
    }
}

impl ECScalar for StarknetCurveScalar {
    type SecretKey = SK;

    fn new_random() -> StarknetCurveScalar {
        use rand::RngCore;
        #[cfg(feature = "wasm")]
        let mut rng = rand::rngs::OsRng;
        #[cfg(not(feature = "wasm"))]
        let mut rng = rand::thread_rng();
        let key = loop {
            let mut ret = [0u8; 32];
            rng.fill_bytes(&mut ret);
            if let Ok(key) = FieldElement::from_bytes_be(&ret) {
                break key;
            }
        };
        StarknetCurveScalar {
            purpose: "random",
            fe: key,
        }
    }

    fn zero() -> StarknetCurveScalar {
        let zero_arr = [0u8; 32];
        let zero = unsafe { std::mem::transmute::<[u8; 32], FieldElement>(zero_arr) };
        StarknetCurveScalar {
            purpose: "zero",
            fe: zero,
        }
    }

    fn is_zero(&self) -> bool {
        self == &Self::zero()
    }

    fn get_element(&self) -> Self::SecretKey {
        self.fe.clone()
    }

    fn set_element(&mut self, element: Self::SecretKey) {
        self.fe = element
    }

    fn from(n: &BigInt) -> StarknetCurveScalar {
        StarknetCurveScalar {
            purpose: "from_big_int",
            fe: FieldElement::from_byte_slice_be(&n.to_bytes()).unwrap(),
        }
    }

    fn to_big_int(&self) -> BigInt {
        BigInt::from_bytes(&self.fe.to_bytes_be())
    }

    fn q() -> BigInt {
        BigInt::from_bytes(&EC_ORDER.to_bytes_be())
    }

    fn add(&self, other: &Self::SecretKey) -> StarknetCurveScalar {
        StarknetCurveScalar {
            purpose: "add",
            fe: self.fe.add(*other),
        }
    }

    fn mul(&self, other: &Self::SecretKey) -> StarknetCurveScalar {
        StarknetCurveScalar {
            purpose: "mul",
            fe: self.fe.mul(*other),
        }
    }

    fn sub(&self, other: &Self::SecretKey) -> StarknetCurveScalar {
        StarknetCurveScalar {
            purpose: "sub",
            fe: self.fe.sub(*other),
        }
    }

    fn invert(&self) -> StarknetCurveScalar {
        StarknetCurveScalar {
            purpose: "invert",
            fe: self.fe.invert().unwrap(),
        }
    }
}

impl Mul<StarknetCurveScalar> for StarknetCurveScalar {
    type Output = StarknetCurveScalar;
    fn mul(self, other: StarknetCurveScalar) -> StarknetCurveScalar {
        (&self).mul(&other.get_element())
    }
}

impl<'o> Mul<&'o StarknetCurveScalar> for StarknetCurveScalar {
    type Output = StarknetCurveScalar;
    fn mul(self, other: &'o StarknetCurveScalar) -> StarknetCurveScalar {
        (&self).mul(&other.get_element())
    }
}

impl Add<StarknetCurveScalar> for StarknetCurveScalar {
    type Output = StarknetCurveScalar;
    fn add(self, other: StarknetCurveScalar) -> StarknetCurveScalar {
        (&self).add(&other.get_element())
    }
}

impl<'o> Add<&'o StarknetCurveScalar> for StarknetCurveScalar {
    type Output = StarknetCurveScalar;
    fn add(self, other: &'o StarknetCurveScalar) -> StarknetCurveScalar {
        (&self).add(&other.get_element())
    }
}

impl Serialize for StarknetCurveScalar {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        serializer.serialize_str(&self.to_big_int().to_hex())
    }
}

impl<'de> Deserialize<'de> for StarknetCurveScalar {
    fn deserialize<D>(deserializer: D) -> Result<StarknetCurveScalar, D::Error>
        where
            D: Deserializer<'de>,
    {
        deserializer.deserialize_str(StarknetCurveScalarVisitor)
    }
}

struct StarknetCurveScalarVisitor;

impl<'de> Visitor<'de> for StarknetCurveScalarVisitor {
    type Value = StarknetCurveScalar;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("StarknetCurveScalar")
    }

    fn visit_str<E: de::Error>(self, s: &str) -> Result<StarknetCurveScalar, E> {
        let v = BigInt::from_hex(s).map_err(E::custom)?;
        Ok(ECScalar::from(&v))
    }
}

impl PartialEq for StarknetCurveScalar {
    fn eq(&self, other: &StarknetCurveScalar) -> bool {
        self.get_element() == other.get_element()
    }
}

impl PartialEq for StarknetCurvePoint {
    fn eq(&self, other: &StarknetCurvePoint) -> bool {
        self.get_element() == other.get_element()
    }
}

impl Zeroize for StarknetCurvePoint {
    fn zeroize(&mut self) {
        unsafe { ptr::write_volatile(self, GE::generator()) };
        atomic::fence(atomic::Ordering::SeqCst);
        atomic::compiler_fence(atomic::Ordering::SeqCst);
    }
}

impl ECPoint for StarknetCurvePoint {
    type SecretKey = SK;
    type PublicKey = Option<PK>;
    type Scalar = StarknetCurveScalar;

    fn zero() -> StarknetCurvePoint {
        StarknetCurvePoint {
            purpose: "zero",
            ge: None,
        }
    }

    fn is_zero(&self) -> bool {
        self.ge.is_none()
    }

    // TODO this is not correct base_point2 like secp256k1, we do not use this.
    fn base_point2() -> StarknetCurvePoint {
        StarknetCurvePoint {
            purpose: "base_point2",
            ge: Some(PEDERSEN_P0),
        }
    }

    fn generator() -> StarknetCurvePoint {
        StarknetCurvePoint {
            purpose: "base_fe",
            ge: Some(GENERATOR),
        }
    }

    fn get_element(&self) -> Self::PublicKey {
        self.ge
    }

    /// to return from BigInt to PK use from_bytes:
    /// 1) convert BigInt::to_vec
    /// 2) call FieldElement::from_byte_slice_be
    fn bytes_compressed_to_big_int(&self) -> BigInt {
        match self.ge {
            Some(ge) => BigInt::from_bytes(&ge.x.to_bytes_be()),
            None => BigInt::zero(),
        }
    }

    fn x_coor(&self) -> Option<BigInt> {
        match self.ge {
            Some(ge) => Some(BigInt::from_bytes(&ge.x.to_bytes_be())),
            None => None,
        }
    }

    fn y_coor(&self) -> Option<BigInt> {
        match self.ge {
            Some(ge) => Some(BigInt::from_bytes(&ge.y.to_bytes_be())),
            None => None,
        }
    }

    fn from_bytes(bytes: &[u8]) -> Result<StarknetCurvePoint, ErrorKey> {
        if bytes.len() != 32usize {
            return Err(ErrorKey::InvalidPublicKey);
        }
        let x = FieldElement::from_byte_slice_be(&bytes).map_err(|_| ErrorKey::InvalidPublicKey)?;
        let ge = AffinePoint::from_x(x).ok_or_else(|| ErrorKey::InvalidPublicKey)?;
        Ok(StarknetCurvePoint {
            purpose: "from_bytes",
            ge: Some(ge),
        })
    }

    fn pk_to_key_slice(&self) -> Vec<u8> {
        match self.ge {
            None => [0u8; 32].to_vec(),
            Some(_ge) => _ge.x.to_bytes_be().to_vec(),
        }
    }

    fn scalar_mul(&self, fe: &Self::SecretKey) -> StarknetCurvePoint {
        if let None = self.ge {
            return self.clone();
        }
        let x = ProjectivePoint::from_affine_point(&self.ge.unwrap());
        let y = fe.to_bits_le();
        let z = &x * &y;
        let p = AffinePoint::from(&z);
        StarknetCurvePoint {
            purpose: "scalar_mul",
            ge: AffinePoint::from_x(p.x),
        }
    }

    fn add_point(&self, other: &Self::PublicKey) -> StarknetCurvePoint {
        StarknetCurvePoint {
            purpose: "add_point",
            ge: match (&self.ge, other) {
                (None, right) => *right,
                (left, None) => *left,
                (Some(left), Some(right)) => Some(left.add(right)),
            },
        }
    }

    fn sub_point(&self, other: &Self::PublicKey) -> StarknetCurvePoint {
        StarknetCurvePoint {
            purpose: "sub_point",
            ge: match (&self.ge, other) {
                (None, Some(ge)) => Some(AffinePoint {
                    x: ge.x,
                    y: -ge.y,
                    infinity: ge.infinity,
                }),
                (Some(ge), None) => Some(*ge),
                (None, None) => None,
                (Some(ge1), Some(ge2)) => {
                    if ge1.x == ge2.x {
                        None
                    } else {
                        Some(ge1.sub(ge2))
                    }
                },
            },
        }
    }

    fn from_coor(x: &BigInt, y: &BigInt) -> StarknetCurvePoint {
        StarknetCurvePoint {
            purpose: "from_coor",
            ge: Some(AffinePoint {
                x: FieldElement::from_byte_slice_be(&BigInt::to_bytes(x)).unwrap(),
                y: FieldElement::from_byte_slice_be(&BigInt::to_bytes(y)).unwrap(),
                infinity: false,
            }),
        }
    }
}

impl Mul<StarknetCurveScalar> for StarknetCurvePoint {
    type Output = StarknetCurvePoint;
    fn mul(self, other: StarknetCurveScalar) -> Self::Output {
        self.scalar_mul(&other.get_element())
    }
}

impl<'o> Mul<&'o StarknetCurveScalar> for StarknetCurvePoint {
    type Output = StarknetCurvePoint;
    fn mul(self, other: &'o StarknetCurveScalar) -> Self::Output {
        self.scalar_mul(&other.get_element())
    }
}

impl<'o> Mul<&'o StarknetCurveScalar> for &'o StarknetCurvePoint {
    type Output = StarknetCurvePoint;
    fn mul(self, other: &'o StarknetCurveScalar) -> Self::Output {
        self.scalar_mul(&other.get_element())
    }
}

impl Add<StarknetCurvePoint> for StarknetCurvePoint {
    type Output = StarknetCurvePoint;
    fn add(self, other: StarknetCurvePoint) -> Self::Output {
        self.add_point(&other.get_element())
    }
}

impl<'o> Add<&'o StarknetCurvePoint> for StarknetCurvePoint {
    type Output = StarknetCurvePoint;
    fn add(self, other: &'o StarknetCurvePoint) -> Self::Output {
        self.add_point(&other.get_element())
    }
}

impl<'o> Add<&'o StarknetCurvePoint> for &'o StarknetCurvePoint {
    type Output = StarknetCurvePoint;
    fn add(self, other: &'o StarknetCurvePoint) -> Self::Output {
        self.add_point(&other.get_element())
    }
}

impl Serialize for StarknetCurvePoint {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        let mut state = serializer.serialize_struct("StarknetCurvePoint", 2)?;
        state.serialize_field("x", &self.x_coor().unwrap().to_hex())?;
        state.serialize_field("y", &self.y_coor().unwrap().to_hex())?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for StarknetCurvePoint {
    fn deserialize<D>(deserializer: D) -> Result<StarknetCurvePoint, D::Error>
        where
            D: Deserializer<'de>,
    {
        let fields = &["x", "y"];
        deserializer.deserialize_struct("StarknetCurvePoint", fields, StarknetCurvePointVisitor)
    }
}

struct StarknetCurvePointVisitor;

impl<'de> Visitor<'de> for StarknetCurvePointVisitor {
    type Value = StarknetCurvePoint;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("StarknetCurvePoint")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<StarknetCurvePoint, V::Error>
        where
            V: SeqAccess<'de>,
    {
        let x = seq
            .next_element()?
            .ok_or_else(|| V::Error::invalid_length(0, &"a single element"))?;
        let y = seq
            .next_element()?
            .ok_or_else(|| V::Error::invalid_length(0, &"a single element"))?;

        let bx = BigInt::from_hex(x).map_err(V::Error::custom)?;
        let by = BigInt::from_hex(y).map_err(V::Error::custom)?;

        Ok(StarknetCurvePoint::from_coor(&bx, &by))
    }

    fn visit_map<E: MapAccess<'de>>(self, mut map: E) -> Result<StarknetCurvePoint, E::Error> {
        let mut x = String::new();
        let mut y = String::new();

        while let Some(ref key) = map.next_key::<String>()? {
            let v = map.next_value::<String>()?;
            if key == "x" {
                x = v
            } else if key == "y" {
                y = v
            } else {
                return Err(E::Error::unknown_field(key, &["x", "y"]));
            }
        }

        let bx = BigInt::from_hex(&x).map_err(E::Error::custom)?;
        let by = BigInt::from_hex(&y).map_err(E::Error::custom)?;

        Ok(StarknetCurvePoint::from_coor(&bx, &by))
    }
}

/// The order of the stark curve(not montgomery)
pub const CURVE_ORDER: [u8; 32] = [
    0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
];

/// The X coordinate of the generator(not montgomery)
pub const GENERATOR_X: [u8; 32] = [
    0x01, 0xef, 0x15, 0xc1, 0x85, 0x99, 0x97, 0x1b, 0x7b, 0xec, 0xed, 0x41, 0x5a, 0x40, 0xf0, 0xc7,
    0xde, 0xac, 0xfd, 0x9b, 0x0d, 0x18, 0x19, 0xe0, 0x3d, 0x72, 0x3d, 0x8b, 0xc9, 0x43, 0xcf, 0xca,
];

/// The Y coordinate of the generator(not montgomery)
pub const GENERATOR_Y: [u8; 32] = [
    0x00, 0x56, 0x68, 0x06, 0x0a, 0xa4, 0x97, 0x30, 0xb7, 0xbe, 0x48, 0x01, 0xdf, 0x46, 0xec, 0x62,
    0xde, 0x53, 0xec, 0xd1, 0x1a, 0xbe, 0x43, 0xa3, 0x28, 0x73, 0x00, 0x0c, 0x36, 0xe8, 0xdc, 0x1f,
];

/// The size (in bytes) of a secret key
pub const SECRET_KEY_SIZE: usize = 32;

/// The size (in bytes) of a serialized public key.
pub const PUBLIC_KEY_SIZE: usize = 32;

#[cfg(test)]
mod tests {
    use std::ops::Neg;
    use super::BigInt;
    use super::StarknetCurvePoint;
    use super::StarknetCurveScalar;
    use super::{GE, FE};
    use crate::arithmetic::traits::*;
    use crate::cryptographic_primitives::hashing::hash_sha256::HSha256;
    use crate::cryptographic_primitives::hashing::traits::Hash;
    use crate::elliptic::curves::traits::ECPoint;
    use crate::elliptic::curves::traits::ECScalar;

    #[test]
    fn stark_is_zero_scalar() {
        let f_l = StarknetCurveScalar::new_random();
        let f_r = f_l.clone();
        let f_s = f_l.sub(&f_r.get_element());
        assert!(!f_l.is_zero());
        assert!(f_s.is_zero());

        let p_l = StarknetCurvePoint::generator();
        let p_r = p_l.clone();
        let p_s = p_l.sub_point(&p_r.get_element());
        assert!(p_s.is_zero());
    }

    #[test]
    fn stark_test_zero_point() {
        let rand_point = StarknetCurvePoint::random_point();
        let x_coor = rand_point.x_coor().unwrap();
        let y_coor1 = rand_point.y_coor().unwrap();
        let y_coor2 = BigInt::from_bytes(&rand_point.ge.unwrap().y.neg().to_bytes_be());
        let point1 = StarknetCurvePoint::from_coor(&x_coor, &y_coor1);
        let point2 = StarknetCurvePoint::from_coor(&x_coor, &y_coor2);

        //add point should not panic
        let point3 = point1.add_point(&point2.get_element());
        println!("get zero point: {:?}", point3);
        println!("zero point serialized: {:?}", point3.pk_to_key_slice());
        let point4 = GE::from_coor(&BigInt::zero(), &BigInt::zero());
        println!("point from zero bigint: {:?}", point4);
        let point5 = GE::from_bytes(&[4; 32]).unwrap();
        println!("zero point from &[u8]: {:?}", point5);
        println!(
            "zero point mul scalar: {:?}",
            point5.scalar_mul(&StarknetCurveScalar::new_random().get_element())
        );
    }

    #[test]
    fn stark_serialize_sk() {
        let scalar: StarknetCurveScalar = ECScalar::from(&BigInt::from(123456));
        let s = serde_json::to_string(&scalar).expect("Failed in serialization");
        assert_eq!(s, "\"1e240\"");
    }

    #[test]
    fn stark_serialize_rand_pk_verify_pad() {
        let r = StarknetCurvePoint::random_point();
        let r_expected = StarknetCurvePoint::from_coor(&r.x_coor().unwrap(), &r.y_coor().unwrap());

        assert_eq!(r.x_coor().unwrap(), r_expected.x_coor().unwrap());
        assert_eq!(r.y_coor().unwrap(), r_expected.y_coor().unwrap());
    }

    #[test]
    fn stark_deserialize_sk() {
        let s = "\"1e240\"";
        let dummy: StarknetCurveScalar = serde_json::from_str(s).expect("Failed in serialization");

        let sk: StarknetCurveScalar = ECScalar::from(&BigInt::from(123456));

        assert_eq!(dummy, sk);
    }

    #[test]
    fn stark_serialize_pk() {
        let pk = StarknetCurvePoint::generator();
        let x = pk.x_coor().unwrap();
        let y = pk.y_coor().unwrap();
        let s = serde_json::to_string(&pk).expect("Failed in serialization");

        let expected = format!("{{\"x\":\"{}\",\"y\":\"{}\"}}", x.to_hex(), y.to_hex());
        assert_eq!(s, expected);

        let des_pk: StarknetCurvePoint = serde_json::from_str(&s).expect("Failed in serialization");
        assert_eq!(des_pk.ge, pk.ge);
    }

    #[test]
    fn stark_bincode_pk() {
        let pk = StarknetCurvePoint::generator();
        let bin = bincode::serialize(&pk).unwrap();
        let decoded: StarknetCurvePoint = bincode::deserialize(bin.as_slice()).unwrap();
        assert_eq!(decoded, pk);
    }

    use crate::ErrorKey;

    #[test]
    fn stark_test_serdes_pk() {
        let pk = GE::generator();
        let s = serde_json::to_string(&pk).expect("Failed in serialization");
        let des_pk: GE = serde_json::from_str(&s).expect("Failed in deserialization");
        assert_eq!(des_pk, pk);

        let pk = GE::base_point2();
        let s = serde_json::to_string(&pk).expect("Failed in serialization");
        let des_pk: GE = serde_json::from_str(&s).expect("Failed in deserialization");
        assert_eq!(des_pk, pk);
    }

    #[test]
    #[should_panic]
    fn stark_test_serdes_bad_pk() {
        let pk = GE::generator();
        let s = serde_json::to_string(&pk).expect("Failed in serialization");
        // we make sure that the string encodes invalid point:
        let s: String = s.replace("d8bc", "d8b3");
        let des_pk: GE = serde_json::from_str(&s).expect("Failed in deserialization");
        assert_eq!(des_pk, pk);
    }

    #[test]
    fn stark_test_from_bytes() {
        let g = StarknetCurvePoint::generator();
        let hash = HSha256::create_hash(&[&g.bytes_compressed_to_big_int()]);
        let hash_vec = BigInt::to_bytes(&hash);
        let result = StarknetCurvePoint::from_bytes(&hash_vec);
        assert_eq!(result.unwrap_err(), ErrorKey::InvalidPublicKey)
    }

    #[test]
    fn stark_test_from_bytes_3() {
        let test_vec = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 1, 2, 3, 4, 5, 6,
        ];
        let result = StarknetCurvePoint::from_bytes(&test_vec);
        assert!(result.is_ok() | result.is_err())
    }

    #[test]
    fn stark_test_from_bytes_4() {
        let test_vec = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 5, 6,
        ];
        let result = StarknetCurvePoint::from_bytes(&test_vec);
        assert!(result.is_ok() | result.is_err())
    }

    #[test]
    fn stark_test_from_bytes_5() {
        let test_vec = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 5,
            6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4,
            5, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3,
            4, 5, 6,
        ];
        let result = StarknetCurvePoint::from_bytes(&test_vec);
        assert!(result.is_ok() | result.is_err())
    }

    #[test]
    fn stark_test_scalar_mul_scalar() {
        let a: FE = ECScalar::new_random();
        let b: FE = ECScalar::new_random();
        let c1 = a.mul(&b.get_element());
        let c2 = a * b;
        assert_eq!(c1.get_element(), c2.get_element());
    }

    #[test]
    fn stark_test_pk_to_key_slice() {
        for _ in 1..200 {
            let r = FE::new_random();
            let rg = GE::generator() * r;
            let key_slice = rg.pk_to_key_slice();
            assert!(key_slice.len() == 32);

            let rg_prime: GE = ECPoint::from_bytes(&key_slice).unwrap();
            assert_eq!(rg_prime.get_element(), rg.get_element());
        }
    }
}