// Copyright (C) 2019-2021 Aleo Systems Inc.
// This file is part of the snarkVM library.

// The snarkVM library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkVM library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkVM library. If not, see <https://www.gnu.org/licenses/>.

//! A sparse polynomial represented in coefficient form.

use crate::fft::{DenseOrSparsePolynomial, EvaluationDomain, Evaluations};
use snarkvm_fields::{Field, PrimeField};
use snarkvm_utilities::{errors::SerializationError, serialize::*};

use std::fmt;

/// Stores a sparse polynomial in coefficient form.
#[derive(Clone, PartialEq, Eq, Hash, Default, CanonicalSerialize, CanonicalDeserialize)]
pub struct SparsePolynomial<F: Field> {
    /// The coefficient a_i of `x^i` is stored as (i, a_i) in `self.coeffs`.
    /// the entries in `self.coeffs` are sorted in increasing order of `i`.
    pub coeffs: Vec<(usize, F)>,
}

impl<F: Field> fmt::Debug for SparsePolynomial<F> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        for (i, coeff) in self.coeffs.iter().filter(|(_, c)| !c.is_zero()) {
            if *i == 0 {
                write!(f, "\n{:?}", coeff)?;
            } else if *i == 1 {
                write!(f, " + \n{:?} * x", coeff)?;
            } else {
                write!(f, " + \n{:?} * x^{}", coeff, i)?;
            }
        }
        Ok(())
    }
}

impl<F: Field> SparsePolynomial<F> {
    /// Returns the zero polynomial.
    pub fn zero() -> Self {
        Self { coeffs: Vec::new() }
    }

    /// Checks if the given polynomial is zero.
    pub fn is_zero(&self) -> bool {
        self.coeffs.is_empty() || self.coeffs.iter().all(|(_, c)| c.is_zero())
    }

    /// Constructs a new polynomial from a list of coefficients.
    pub fn from_coefficients_slice(coeffs: &[(usize, F)]) -> Self {
        Self::from_coefficients_vec(coeffs.to_vec())
    }

    /// Constructs a new polynomial from a list of coefficients.
    pub fn from_coefficients_vec(mut coeffs: Vec<(usize, F)>) -> Self {
        // While there are zeros at the end of the coefficient vector, pop them off.
        while coeffs.last().map_or(false, |(_, c)| c.is_zero()) {
            coeffs.pop();
        }
        // Check that either the coefficients vec is empty or that the last coeff is non-zero.
        assert!(coeffs.last().map_or(true, |(_, c)| !c.is_zero()));

        Self { coeffs }
    }

    /// Returns the degree of the polynomial.
    pub fn degree(&self) -> usize {
        if self.is_zero() {
            0
        } else {
            assert!(self.coeffs.last().map_or(false, |(_, c)| !c.is_zero()));
            self.coeffs.last().unwrap().0
        }
    }

    /// Evaluates `self` at the given `point` in the field.
    pub fn evaluate(&self, point: F) -> F {
        if self.is_zero() {
            return F::zero();
        }
        let mut total = F::zero();
        for (i, c) in &self.coeffs {
            total += *c * point.pow(&[*i as u64]);
        }
        total
    }

    /// Perform a naive n^2 multiplicatoin of `self` by `other`.
    pub fn mul(&self, other: &Self) -> Self {
        if self.is_zero() || other.is_zero() {
            SparsePolynomial::zero()
        } else {
            let mut result = std::collections::HashMap::new();
            for (i, self_coeff) in self.coeffs.iter() {
                for (j, other_coeff) in other.coeffs.iter() {
                    let cur_coeff = result.entry(i + j).or_insert_with(F::zero);
                    *cur_coeff += *self_coeff * other_coeff;
                }
            }
            let mut result = result.into_iter().collect::<Vec<_>>();
            result.sort_by(|a, b| a.0.cmp(&b.0));
            SparsePolynomial::from_coefficients_vec(result)
        }
    }
}

impl<F: PrimeField> SparsePolynomial<F> {
    /// Evaluate `self` over `domain`.
    pub fn evaluate_over_domain_by_ref(&self, domain: EvaluationDomain<F>) -> Evaluations<F> {
        let poly: DenseOrSparsePolynomial<'_, F> = self.into();
        DenseOrSparsePolynomial::<F>::evaluate_over_domain(poly, domain)
        // unimplemented!("current implementation does not produce evals in correct order")
    }

    /// Evaluate `self` over `domain`.
    pub fn evaluate_over_domain(self, domain: EvaluationDomain<F>) -> Evaluations<F> {
        let poly: DenseOrSparsePolynomial<'_, F> = self.into();
        DenseOrSparsePolynomial::<F>::evaluate_over_domain(poly, domain)
        // unimplemented!("current implementation does not produce evals in correct order")
    }
}
impl<F: PrimeField> core::ops::MulAssign<F> for SparsePolynomial<F> {
    fn mul_assign(&mut self, other: F) {
        for (_, coeff) in &mut self.coeffs {
            *coeff *= other;
        }
    }
}

impl<'a, F: PrimeField> core::ops::Mul<F> for &'a SparsePolynomial<F> {
    type Output = SparsePolynomial<F>;

    fn mul(self, other: F) -> Self::Output {
        let mut result = self.clone();
        result *= other;
        result
    }
}

#[cfg(test)]
mod tests {
    use crate::fft::{DensePolynomial, EvaluationDomain, SparsePolynomial};
    use snarkvm_curves::bls12_377::Fr;
    use snarkvm_fields::One;

    #[test]
    fn evaluate_over_domain() {
        for size in 2..10 {
            let domain_size = 1 << size;
            let domain = EvaluationDomain::new(domain_size).unwrap();
            let two = Fr::one() + Fr::one();
            let sparse_poly = SparsePolynomial::from_coefficients_vec(vec![(0, two), (1, two)]);
            let evals1 = sparse_poly.evaluate_over_domain_by_ref(domain);

            let dense_poly: DensePolynomial<Fr> = sparse_poly.into();
            let evals2 = dense_poly.clone().evaluate_over_domain(domain);
            assert_eq!(evals1.clone().interpolate(), evals2.clone().interpolate());
            assert_eq!(evals1.interpolate(), dense_poly);
            assert_eq!(evals2.interpolate(), dense_poly);
        }
    }
}
