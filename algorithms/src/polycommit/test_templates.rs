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

use super::*;
use crate::fft::DensePolynomial;
use snarkvm_utilities::rand::test_rng;

use rand::{distributions::Distribution, Rng};

#[derive(Default)]
struct TestInfo {
    num_iters: usize,
    max_degree: Option<usize>,
    supported_degree: Option<usize>,
    num_polynomials: usize,
    enforce_degree_bounds: bool,
    max_num_queries: usize,
    num_equations: Option<usize>,
}

pub struct TestComponents<F: PrimeField, CF: PrimeField, PC: PolynomialCommitment<F, CF>> {
    pub verification_key: PC::VerifierKey,
    pub commitments: Vec<LabeledCommitment<PC::Commitment>>,
    pub query_set: QuerySet<'static, F>,
    pub evaluations: Evaluations<'static, F>,
    pub batch_lc_proof: Option<BatchLCProof<F, CF, PC>>,
    pub batch_proof: Option<PC::BatchProof>,
    pub opening_challenge: F,
    pub randomness: Vec<PC::Randomness>,
}

pub fn bad_degree_bound_test<F, CF, PC>() -> Result<(), PCError>
where
    F: PrimeField,
    CF: PrimeField,
    PC: PolynomialCommitment<F, CF>,
{
    let rng = &mut test_rng();
    let max_degree = 100;
    let pp = PC::setup(max_degree, rng)?;

    for _ in 0..10 {
        let supported_degree = rand::distributions::Uniform::from(1..=max_degree).sample(rng);
        assert!(max_degree >= supported_degree, "max_degree < supported_degree");

        let mut labels = Vec::new();
        let mut polynomials = Vec::new();
        let mut degree_bounds = Vec::new();

        for i in 0..10 {
            let label = format!("Test{}", i);
            labels.push(label.clone());
            let poly = DensePolynomial::rand(supported_degree, rng);

            let degree_bound = 1usize;
            let hiding_bound = Some(1);
            degree_bounds.push(degree_bound);

            polynomials.push(LabeledPolynomial::new(label, poly, Some(degree_bound), hiding_bound))
        }

        println!("supported degree: {:?}", supported_degree);
        let (ck, vk) = PC::trim(&pp, supported_degree, supported_degree, Some(degree_bounds.as_slice()))?;
        println!("Trimmed");

        let (comms, rands) = PC::commit(&ck, &polynomials, Some(rng))?;

        let mut query_set = QuerySet::new();
        let mut values = Evaluations::new();
        let point = F::rand(rng);
        for (i, label) in labels.iter().enumerate() {
            query_set.insert((label.clone(), ("rand".into(), point)));
            let value = polynomials[i].evaluate(point);
            values.insert((label.clone(), point), value);
        }
        println!("Generated query set");

        let opening_challenge = F::rand(rng);
        let proof = PC::batch_open(&ck, &polynomials, &comms, &query_set, opening_challenge, &rands, Some(rng))?;
        let result = PC::batch_check(&vk, &comms, &query_set, &values, &proof, opening_challenge, rng)?;
        assert!(result, "proof was incorrect, Query set: {:#?}", query_set);
    }
    Ok(())
}

fn test_template<F, CF, PC>(info: TestInfo) -> Result<Vec<TestComponents<F, CF, PC>>, PCError>
where
    F: PrimeField,
    CF: PrimeField,
    PC: PolynomialCommitment<F, CF>,
{
    let TestInfo {
        num_iters,
        max_degree,
        supported_degree,
        num_polynomials,
        enforce_degree_bounds,
        max_num_queries,
        ..
    } = info;

    let mut test_components = Vec::new();

    let rng = &mut test_rng();
    let max_degree = max_degree.unwrap_or_else(|| rand::distributions::Uniform::from(8..=64).sample(rng));
    let pp = PC::setup(max_degree, rng)?;
    let supported_degree_bounds = pp.supported_degree_bounds();

    for _ in 0..num_iters {
        let supported_degree =
            supported_degree.unwrap_or_else(|| rand::distributions::Uniform::from(4..=max_degree).sample(rng));
        assert!(max_degree >= supported_degree, "max_degree < supported_degree");
        let mut polynomials = Vec::new();
        let mut degree_bounds = if enforce_degree_bounds { Some(Vec::new()) } else { None };

        let mut labels = Vec::new();
        println!("Sampled supported degree");

        // Generate polynomials
        let num_points_in_query_set = rand::distributions::Uniform::from(1..=max_num_queries).sample(rng);
        for i in 0..num_polynomials {
            let label = format!("Test{}", i);
            labels.push(label.clone());
            let degree = rand::distributions::Uniform::from(1..=supported_degree).sample(rng);
            let poly = DensePolynomial::rand(degree, rng);

            let supported_degree_bounds_after_trimmed = supported_degree_bounds
                .iter()
                .copied()
                .filter(|x| *x >= degree && *x < supported_degree)
                .collect::<Vec<usize>>();

            let degree_bound = if let Some(degree_bounds) = &mut degree_bounds {
                if !supported_degree_bounds_after_trimmed.is_empty() && rng.gen() {
                    let range = rand::distributions::Uniform::from(0..supported_degree_bounds_after_trimmed.len());
                    let idx = range.sample(rng);

                    let degree_bound = supported_degree_bounds_after_trimmed[idx];
                    degree_bounds.push(degree_bound);
                    Some(degree_bound)
                } else {
                    None
                }
            } else {
                None
            };

            let hiding_bound =
                if num_points_in_query_set >= degree { Some(degree) } else { Some(num_points_in_query_set) };
            println!("Hiding bound: {:?}", hiding_bound);

            polynomials.push(LabeledPolynomial::new(label, poly, degree_bound, hiding_bound))
        }
        let supported_hiding_bound = polynomials.iter().map(|p| p.hiding_bound().unwrap_or(0)).max().unwrap_or(0);
        println!("supported degree: {:?}", supported_degree);
        println!("supported hiding bound: {:?}", supported_hiding_bound);
        println!("num_points_in_query_set: {:?}", num_points_in_query_set);
        let (ck, vk) = PC::trim(&pp, supported_degree, supported_hiding_bound, degree_bounds.as_deref())?;
        println!("Trimmed");

        let (comms, rands) = PC::commit(&ck, &polynomials, Some(rng))?;

        // Construct query set
        let mut query_set = QuerySet::new();
        let mut values = Evaluations::new();
        // let mut point = F::one();
        for point_id in 0..num_points_in_query_set {
            let point = F::rand(rng);
            for (polynomial, label) in polynomials.iter().zip(labels.iter()) {
                query_set.insert((label.clone(), (format!("rand_{}", point_id), point)));
                let value = polynomial.evaluate(point);
                values.insert((label.clone(), point), value);
            }
        }
        println!("Generated query set");

        let opening_challenge = F::rand(rng);
        let proof = PC::batch_open(&ck, &polynomials, &comms, &query_set, opening_challenge, &rands, Some(rng))?;
        let result = PC::batch_check(&vk, &comms, &query_set, &values, &proof, opening_challenge, rng)?;
        if !result {
            println!(
                "Failed with {} polynomials, num_points_in_query_set: {:?}",
                num_polynomials, num_points_in_query_set
            );
            println!("Degree of polynomials:",);
            for poly in polynomials {
                println!("Degree: {:?}", poly.degree());
            }
        }
        assert!(result, "proof was incorrect, Query set: {:#?}", query_set);

        test_components.push(TestComponents {
            verification_key: vk,
            commitments: comms,
            query_set,
            evaluations: values,
            batch_lc_proof: None,
            batch_proof: Some(proof),
            opening_challenge,
            randomness: rands,
        });
    }
    Ok(test_components)
}

fn equation_test_template<F, CF, PC>(info: TestInfo) -> Result<Vec<TestComponents<F, CF, PC>>, PCError>
where
    F: PrimeField,
    CF: PrimeField,
    PC: PolynomialCommitment<F, CF>,
{
    let TestInfo {
        num_iters,
        max_degree,
        supported_degree,
        num_polynomials,
        enforce_degree_bounds,
        max_num_queries,
        num_equations,
    } = info;

    let mut test_components = Vec::new();

    let rng = &mut test_rng();
    let max_degree = max_degree.unwrap_or_else(|| rand::distributions::Uniform::from(8..=64).sample(rng));
    let pp = PC::setup(max_degree, rng)?;
    let supported_degree_bounds = pp.supported_degree_bounds();

    for _ in 0..num_iters {
        let supported_degree =
            supported_degree.unwrap_or_else(|| rand::distributions::Uniform::from(4..=max_degree).sample(rng));
        assert!(max_degree >= supported_degree, "max_degree < supported_degree");
        let mut polynomials = Vec::new();
        let mut degree_bounds = if enforce_degree_bounds { Some(Vec::new()) } else { None };

        let mut labels = Vec::new();
        println!("Sampled supported degree");

        // Generate polynomials
        let num_points_in_query_set = rand::distributions::Uniform::from(1..=max_num_queries).sample(rng);
        for i in 0..num_polynomials {
            let label = format!("Test{}", i);
            labels.push(label.clone());
            let degree = rand::distributions::Uniform::from(1..=supported_degree).sample(rng);
            let poly = DensePolynomial::rand(degree, rng);

            let supported_degree_bounds_after_trimmed = supported_degree_bounds
                .iter()
                .copied()
                .filter(|x| *x >= degree && *x < supported_degree)
                .collect::<Vec<usize>>();

            let degree_bound = if let Some(degree_bounds) = &mut degree_bounds {
                if !supported_degree_bounds_after_trimmed.is_empty() && rng.gen() {
                    let range = rand::distributions::Uniform::from(0..supported_degree_bounds_after_trimmed.len());
                    let idx = range.sample(rng);

                    let degree_bound = supported_degree_bounds_after_trimmed[idx];
                    degree_bounds.push(degree_bound);
                    Some(degree_bound)
                } else {
                    None
                }
            } else {
                None
            };

            let hiding_bound =
                if num_points_in_query_set >= degree { Some(degree) } else { Some(num_points_in_query_set) };
            println!("Hiding bound: {:?}", hiding_bound);

            polynomials.push(LabeledPolynomial::new(label, poly, degree_bound, hiding_bound))
        }
        let supported_hiding_bound = polynomials.iter().map(|p| p.hiding_bound().unwrap_or(0)).max().unwrap_or(0);
        println!("supported degree: {:?}", supported_degree);
        println!("supported hiding bound: {:?}", supported_hiding_bound);
        println!("num_points_in_query_set: {:?}", num_points_in_query_set);
        println!("{:?}", degree_bounds);
        println!("{}", num_polynomials);
        println!("{}", enforce_degree_bounds);

        let (ck, vk) = PC::trim(&pp, supported_degree, supported_hiding_bound, degree_bounds.as_deref())?;
        println!("Trimmed");

        let (comms, rands) = PC::commit(&ck, &polynomials, Some(rng))?;

        // Let's construct our equations
        let mut linear_combinations = Vec::new();
        let mut query_set = QuerySet::new();
        let mut values = Evaluations::new();
        for i in 0..num_points_in_query_set {
            let point = F::rand(rng);
            for j in 0..num_equations.unwrap() {
                let label = format!("query {} eqn {}", i, j);
                let mut lc = LinearCombination::empty(label.clone());

                let mut value = F::zero();
                let should_have_degree_bounds: bool = rng.gen();
                for (k, label) in labels.iter().enumerate() {
                    if should_have_degree_bounds {
                        value += &polynomials[k].evaluate(point);
                        lc.push((F::one(), label.to_string().into()));
                        break;
                    } else {
                        let poly = &polynomials[k];
                        if poly.degree_bound().is_some() {
                            continue;
                        } else {
                            assert!(poly.degree_bound().is_none());
                            let coeff = F::rand(rng);
                            value += &(coeff * poly.evaluate(point));
                            lc.push((coeff, label.to_string().into()));
                        }
                    }
                }
                values.insert((label.clone(), point), value);
                if !lc.is_empty() {
                    linear_combinations.push(lc);
                    // Insert query
                    query_set.insert((label.clone(), (format!("rand_{}", i), point)));
                }
            }
        }
        if linear_combinations.is_empty() {
            continue;
        }
        println!("Generated query set");
        println!("Linear combinations: {:?}", linear_combinations);

        let opening_challenge = F::rand(rng);
        let proof = PC::open_combinations(
            &ck,
            &linear_combinations,
            &polynomials,
            &comms,
            &query_set,
            opening_challenge,
            &rands,
            Some(rng),
        )?;
        println!("Generated proof");
        let result = PC::check_combinations(
            &vk,
            &linear_combinations,
            &comms,
            &query_set,
            &values,
            &proof,
            opening_challenge,
            rng,
        )?;
        if !result {
            println!(
                "Failed with {} polynomials, num_points_in_query_set: {:?}",
                num_polynomials, num_points_in_query_set
            );
            println!("Degree of polynomials:",);
            for poly in polynomials {
                println!("Degree: {:?}", poly.degree());
            }
        }
        assert!(result, "proof was incorrect, equations: {:#?}", linear_combinations);

        test_components.push(TestComponents {
            verification_key: vk,
            commitments: comms,
            query_set,
            evaluations: values,
            batch_lc_proof: Some(proof),
            batch_proof: None,
            opening_challenge,
            randomness: rands,
        });
    }
    Ok(test_components)
}

pub fn single_poly_test<F, CF, PC>() -> Result<Vec<TestComponents<F, CF, PC>>, PCError>
where
    F: PrimeField,
    CF: PrimeField,
    PC: PolynomialCommitment<F, CF>,
{
    let info = TestInfo {
        num_iters: 100,
        max_degree: None,
        supported_degree: None,
        num_polynomials: 1,
        enforce_degree_bounds: false,
        max_num_queries: 1,
        ..Default::default()
    };
    test_template::<F, CF, PC>(info)
}

pub fn linear_poly_degree_bound_test<F, CF, PC>() -> Result<Vec<TestComponents<F, CF, PC>>, PCError>
where
    F: PrimeField,
    CF: PrimeField,
    PC: PolynomialCommitment<F, CF>,
{
    let info = TestInfo {
        num_iters: 100,
        max_degree: Some(2),
        supported_degree: Some(1),
        num_polynomials: 1,
        enforce_degree_bounds: true,
        max_num_queries: 1,
        ..Default::default()
    };
    test_template::<F, CF, PC>(info)
}

pub fn single_poly_degree_bound_test<F, CF, PC>() -> Result<Vec<TestComponents<F, CF, PC>>, PCError>
where
    F: PrimeField,
    CF: PrimeField,
    PC: PolynomialCommitment<F, CF>,
{
    let info = TestInfo {
        num_iters: 100,
        max_degree: None,
        supported_degree: None,
        num_polynomials: 1,
        enforce_degree_bounds: true,
        max_num_queries: 1,
        ..Default::default()
    };
    test_template::<F, CF, PC>(info)
}

pub fn quadratic_poly_degree_bound_multiple_queries_test<F, CF, PC>() -> Result<Vec<TestComponents<F, CF, PC>>, PCError>
where
    F: PrimeField,
    CF: PrimeField,
    PC: PolynomialCommitment<F, CF>,
{
    let info = TestInfo {
        num_iters: 100,
        max_degree: Some(3),
        supported_degree: Some(2),
        num_polynomials: 1,
        enforce_degree_bounds: true,
        max_num_queries: 2,
        ..Default::default()
    };
    test_template::<F, CF, PC>(info)
}

pub fn single_poly_degree_bound_multiple_queries_test<F, CF, PC>() -> Result<Vec<TestComponents<F, CF, PC>>, PCError>
where
    F: PrimeField,
    CF: PrimeField,
    PC: PolynomialCommitment<F, CF>,
{
    let info = TestInfo {
        num_iters: 100,
        max_degree: None,
        supported_degree: None,
        num_polynomials: 1,
        enforce_degree_bounds: true,
        max_num_queries: 2,
        ..Default::default()
    };
    test_template::<F, CF, PC>(info)
}

pub fn two_polys_degree_bound_single_query_test<F, CF, PC>() -> Result<Vec<TestComponents<F, CF, PC>>, PCError>
where
    F: PrimeField,
    CF: PrimeField,
    PC: PolynomialCommitment<F, CF>,
{
    let info = TestInfo {
        num_iters: 100,
        max_degree: None,
        supported_degree: None,
        num_polynomials: 2,
        enforce_degree_bounds: true,
        max_num_queries: 1,
        ..Default::default()
    };
    test_template::<F, CF, PC>(info)
}

pub fn full_end_to_end_test<F, CF, PC>() -> Result<Vec<TestComponents<F, CF, PC>>, PCError>
where
    F: PrimeField,
    CF: PrimeField,
    PC: PolynomialCommitment<F, CF>,
{
    let info = TestInfo {
        num_iters: 100,
        max_degree: None,
        supported_degree: None,
        num_polynomials: 10,
        enforce_degree_bounds: true,
        max_num_queries: 5,
        ..Default::default()
    };
    test_template::<F, CF, PC>(info)
}

pub fn full_end_to_end_equation_test<F, CF, PC>() -> Result<Vec<TestComponents<F, CF, PC>>, PCError>
where
    F: PrimeField,
    CF: PrimeField,
    PC: PolynomialCommitment<F, CF>,
{
    let info = TestInfo {
        num_iters: 100,
        max_degree: None,
        supported_degree: None,
        num_polynomials: 10,
        enforce_degree_bounds: true,
        max_num_queries: 5,
        num_equations: Some(10),
    };
    equation_test_template::<F, CF, PC>(info)
}

pub fn single_equation_test<F, CF, PC>() -> Result<Vec<TestComponents<F, CF, PC>>, PCError>
where
    F: PrimeField,
    CF: PrimeField,
    PC: PolynomialCommitment<F, CF>,
{
    let info = TestInfo {
        num_iters: 100,
        max_degree: None,
        supported_degree: None,
        num_polynomials: 1,
        enforce_degree_bounds: false,
        max_num_queries: 1,
        num_equations: Some(1),
    };
    equation_test_template::<F, CF, PC>(info)
}

pub fn two_equation_test<F, CF, PC>() -> Result<Vec<TestComponents<F, CF, PC>>, PCError>
where
    F: PrimeField,
    CF: PrimeField,
    PC: PolynomialCommitment<F, CF>,
{
    let info = TestInfo {
        num_iters: 100,
        max_degree: None,
        supported_degree: None,
        num_polynomials: 2,
        enforce_degree_bounds: false,
        max_num_queries: 1,
        num_equations: Some(2),
    };
    equation_test_template::<F, CF, PC>(info)
}

pub fn two_equation_degree_bound_test<F, CF, PC>() -> Result<Vec<TestComponents<F, CF, PC>>, PCError>
where
    F: PrimeField,
    CF: PrimeField,
    PC: PolynomialCommitment<F, CF>,
{
    let info = TestInfo {
        num_iters: 100,
        max_degree: None,
        supported_degree: None,
        num_polynomials: 2,
        enforce_degree_bounds: true,
        max_num_queries: 1,
        num_equations: Some(2),
    };
    equation_test_template::<F, CF, PC>(info)
}
