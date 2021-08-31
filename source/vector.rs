use rstar::Point;
use std::ops::Add;
use std::ops::AddAssign;
use std::ops::Div;
use std::ops::DivAssign;
use std::ops::Index;
use std::ops::IndexMut;
use std::ops::Mul;
use std::ops::MulAssign;
use std::ops::Neg;
use std::ops::Sub;
use std::ops::SubAssign;

/// A three-dimensional vector whose components are f32 values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vector3(pub [f32; 3]);

// Element access.

impl Index<usize> for Vector3 {
    type Output = f32;

    fn index(&self, index: usize) -> &f32 {
        &self.0[index]
    }
}

impl IndexMut<usize> for Vector3 {
    fn index_mut(&mut self, index: usize) -> &mut f32 {
        &mut self.0[index]
    }
}

// Negation (additive inversion).

impl Neg for Vector3 {
    type Output = Self;

    fn neg(self) -> Self {
        Self([
            -self[0],
            -self[1],
            -self[2]
        ])
    }
}

// Vector (element-wise) addition.

impl AddAssign for Vector3 {
    fn add_assign(&mut self, other: Self) {
        *self = Self([
            self[0] + other[0],
            self[1] + other[1],
            self[2] + other[2]
        ]);
    }
}

impl Add for Vector3 {
    type Output = Self;

    fn add(self, addend: Self) -> Self {
        let mut augend = self.clone();
        augend += addend;
        augend
    }
}

impl SubAssign for Vector3 {
    fn sub_assign(&mut self, other: Self) {
        *self = Self([
            self[0] - other[0],
            self[1] - other[1],
            self[2] - other[2]
        ]);
    }
}

impl Sub for Vector3 {
    type Output = Self;

    fn sub(self, addend: Self) -> Self {
        let mut augend = self.clone();
        augend -= addend;
        augend
    }
}

// Scalar multiplication.

impl MulAssign<f32> for Vector3 {
    fn mul_assign(&mut self, other: f32) {
        *self = Self([
            other * self[0],
            other * self[1],
            other * self[2]
        ]);
    }
}

impl Mul<f32> for Vector3 {
    type Output = Self;

    fn mul(self, multiplicand: f32) -> Self {
        let mut multiplier = self.clone();
        multiplier *= multiplicand;
        multiplier
    }
}

// Scalar multiplication is commutative.
impl Mul<Vector3> for f32 {
    type Output = Vector3;

    fn mul(self, multiplicand: Vector3) -> Vector3 {
        let mut multiplicand = multiplicand.clone();
        multiplicand *= self;
        multiplicand
    }
}

impl DivAssign<f32> for Vector3 {
    fn div_assign(&mut self, other: f32) {
        *self = Self([
            self[0] / other,
            self[1] / other,
            self[2] / other
        ]);
    }
}

impl Div<f32> for Vector3 {
    type Output = Self;

    fn div(self, denominator: f32) -> Self {
        let mut numerator = self.clone();
        numerator /= denominator;
        numerator
    }
}

// Hadamard (element-wise) multiplication.

impl MulAssign for Vector3 {
    fn mul_assign(&mut self, other: Self) {
        *self = Self([
            self[0] * other[0],
            self[1] * other[1],
            self[2] * other[2]
        ]);
    }
}

impl Mul for Vector3 {
    type Output = Self;

    fn mul(self, multiplicand: Self) -> Self {
        let mut multiplier = self.clone();
        multiplier *= multiplicand;
        multiplier
    }
}

impl DivAssign for Vector3 {
    fn div_assign(&mut self, other: Self) {
        *self = Self([
            self[0] / other[0],
            self[1] / other[1],
            self[2] / other[2]
        ]);
    }
}

impl Div for Vector3 {
    type Output = Self;

    fn div(self, denominator: Self) -> Self {
        let mut numerator = self.clone();
        numerator /= denominator;
        numerator
    }
}

// Interoperability with rtree.

impl Point for Vector3 {
    type Scalar = f32;
    const DIMENSIONS: usize = 3;

    fn generate(mut generator: impl FnMut(usize) -> Self::Scalar) -> Self {
        Self([
            generator(0),
            generator(1),
            generator(2)
        ])
    }

    fn nth(&self, index: usize) -> f32 {
        self[index]
    }

    fn nth_mut(&mut self, index: usize) -> &mut f32 {
        &mut self[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use test_case::test_case;

    #[test_case(
        Vector3([1.0, -1.0, 1.0]),
        -Vector3([-1.0, 1.0, -1.0])
    ; "negation")]
    #[test_case(
        Vector3([4.0, 4.0, 4.0]),
        Vector3([1.0, 2.0, 3.0]) + Vector3([3.0, 2.0, 1.0])
    ; "vector_addition")]
    #[test_case(
        Vector3([1.0, 2.0, 3.0]),
        Vector3([4.0, 4.0, 4.0]) - Vector3([3.0, 2.0, 1.0])
    ; "vector_addition_inverse")]
    #[test_case(
        Vector3([2.0, 2.0, 2.0]),
        2.0 * Vector3([1.0, 1.0, 1.0])
    ; "scalar_multiplication")]
    #[test_case(
        Vector3([2.0, 2.0, 2.0]),
        Vector3([1.0, 1.0, 1.0]) * 2.0
    ; "scalar_multiplication_commutativity")]
    #[test_case(
        Vector3([0.5, 0.5, 0.5]),
        Vector3([1.0, 1.0, 1.0]) / 2.0
    ; "scalar_multiplication_inverse")]
    #[test_case(
        Vector3([1.0, 4.0, 9.0]),
        Vector3([1.0, 2.0, 3.0]) * Vector3([1.0, 2.0, 3.0])
    ; "hadamard_multiplication")]
    #[test_case(
        Vector3([1.0, 2.0, 3.0]),
        Vector3([1.0, 4.0, 9.0]) / Vector3([1.0, 2.0, 3.0])
    ; "hadamard_multiplication_inverse")]
    fn test_f32_eq(reference: Vector3, result: Vector3) {
        const TOLERANCE: f32 = 0.0005;

        for index in 0..3 {
            assert!((reference[index] - result[index]).abs() <= TOLERANCE,
                "Index {}: {} !~= {} (+/- {})",
                index,
                reference[index],
                result[index],
                TOLERANCE
            );
        }
    }
}