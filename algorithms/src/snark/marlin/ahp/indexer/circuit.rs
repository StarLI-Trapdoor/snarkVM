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

use core::marker::PhantomData;

use crate::{
    polycommit::LabeledPolynomial,
    snark::marlin::{ahp::matrices::MatrixArithmetization, CircuitInfo, MarlinMode, Matrix},
};
use snarkvm_fields::PrimeField;
use snarkvm_utilities::{errors::SerializationError, serialize::*};

#[derive(derivative::Derivative)]
#[derivative(Clone(bound = "F: PrimeField"))]
#[derive(CanonicalSerialize, CanonicalDeserialize, Debug)]
/// The indexed version of the constraint system.
/// This struct contains three kinds of objects:
/// 1) `index_info` is information about the index, such as the size of the
///     public input
/// 2) `{a,b,c}` are the matrices defining the R1CS instance
/// 3) `{a,b,c}_star_arith` are structs containing information about A^*, B^*, and C^*,
/// which are matrices defined as `M^*(i, j) = M(j, i) * u_H(j, j)`.
pub struct Circuit<F: PrimeField, MM: MarlinMode> {
    /// Information about the indexed circuit.
    pub index_info: CircuitInfo<F>,

    /// The A matrix for the R1CS instance
    pub a: Matrix<F>,
    /// The B matrix for the R1CS instance
    pub b: Matrix<F>,
    /// The C matrix for the R1CS instance
    pub c: Matrix<F>,

    /// Joint arithmetization of the A*, B*, and C* matrices.
    pub a_arith: MatrixArithmetization<F>,
    pub b_arith: MatrixArithmetization<F>,
    pub c_arith: MatrixArithmetization<F>,

    pub(crate) mode: PhantomData<MM>,
}

impl<F: PrimeField, MM: MarlinMode> Circuit<F, MM> {
    /// The maximum degree required to represent polynomials of this index.
    pub fn max_degree(&self) -> usize {
        self.index_info.max_degree::<MM>()
    }

    /// Iterate over the indexed polynomials.
    pub fn iter(&self) -> impl Iterator<Item = &LabeledPolynomial<F>> {
        [
            &self.a_arith.row,
            &self.a_arith.col,
            &self.a_arith.val,
            &self.a_arith.row_col,
            &self.b_arith.row,
            &self.b_arith.col,
            &self.b_arith.val,
            &self.b_arith.row_col,
            &self.c_arith.row,
            &self.c_arith.col,
            &self.c_arith.val,
            &self.c_arith.row_col,
        ]
        .into_iter()
    }
}
