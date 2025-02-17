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

use snarkvm_curves::{traits::AffineCurve, Group, ProjectiveCurve};
use snarkvm_fields::{FieldParameters, One, PrimeField, Zero};
use snarkvm_utilities::BigInteger;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

fn update_buckets<G: AffineCurve>(
    base: &G,
    mut scalar: <G::ScalarField as PrimeField>::BigInteger,
    w_start: usize,
    c: usize,
    buckets: &mut [G::Projective],
) {
    // We right-shift by w_start, thus getting rid of the
    // lower bits.
    scalar.divn(w_start as u32);

    // We mod the remaining bits by the window size.
    let scalar = scalar.as_ref()[0] % (1 << c);

    // If the scalar is non-zero, we update the corresponding
    // bucket.
    // (Recall that `buckets` doesn't have a zero bucket.)
    if scalar != 0 {
        buckets[(scalar - 1) as usize].add_assign_mixed(base);
    }
}

fn process_window<G: AffineCurve>(
    bases: &[G],
    scalars: &[<G::ScalarField as PrimeField>::BigInteger],
    w_start: usize,
    c: usize,
) -> G::Projective {
    let mut res = G::Projective::zero();
    let fr_one = G::ScalarField::one().to_repr();

    // We only process unit scalars once in the first window.
    if w_start == 0 {
        scalars.iter().zip(bases).filter(|(&s, _)| s == fr_one).for_each(|(_, base)| {
            res.add_assign_mixed(base);
        });
    }

    // We don't need the "zero" bucket, so we only have 2^c - 1 buckets
    let mut buckets = vec![G::Projective::zero(); (1 << c) - 1];
    scalars
        .iter()
        .zip(bases)
        .filter(|(&s, _)| s > fr_one)
        .for_each(|(&scalar, base)| update_buckets(base, scalar, w_start, c, &mut buckets));
    // G::Projective::batch_normalization(&mut buckets);

    for running_sum in buckets.into_iter().rev().scan(G::Projective::zero(), |sum, b| {
        *sum += b;
        Some(*sum)
    }) {
        res += running_sum;
    }

    res
}

pub fn msm_standard<G: AffineCurve>(
    bases: &[G],
    scalars: &[<G::ScalarField as PrimeField>::BigInteger],
) -> G::Projective {
    let c = if scalars.len() < 32 {
        3
    } else {
        (2.0 / 3.0 * (f64::from(scalars.len() as u32)).log2() + 2.0).ceil() as usize
    };

    let num_bits = <G::ScalarField as PrimeField>::Parameters::MODULUS_BITS as usize;

    // Each window is of size `c`.
    // We divide up the bits 0..num_bits into windows of size `c`, and
    // in parallel process each such window.
    // window_starts.into_iter() //
    let window_sums: Vec<_> = crate::cfg_into_iter!(0..num_bits)
        .step_by(c)
        .map(|w_start| process_window(bases, scalars, w_start, c))
        .collect();

    // We store the sum for the lowest window.
    let (lowest, window_sums) = window_sums.split_first().unwrap();

    // We're traversing windows from high to low.
    window_sums.iter().rev().fold(G::Projective::zero(), |mut total, sum_i| {
        total += sum_i;
        for _ in 0..c {
            total.double_in_place();
        }
        total
    }) + lowest
}
