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

use std::convert::TryInto;

use rand::{Rng, SeedableRng};
use rand_xorshift::XorShiftRng;

use snarkvm_fields::{One, Zero};
use snarkvm_r1cs::{ConstraintSystem, Fr, TestConstraintSystem};

use crate::{
    bits::Boolean,
    integers::uint::{Sub, UInt, UInt64},
    traits::{alloc::AllocGadget, bits::Xor, integers::*},
};

fn check_all_constant_bits(mut expected: u64, actual: UInt64) {
    for b in actual.bits.iter() {
        match *b {
            Boolean::Is(_) => panic!(),
            Boolean::Not(_) => panic!(),
            Boolean::Constant(b) => {
                assert!(b == (expected & 1 == 1));
            }
        }

        expected >>= 1;
    }
}

fn check_all_allocated_bits(mut expected: u64, actual: UInt64) {
    for b in actual.bits.iter() {
        match *b {
            Boolean::Is(ref b) => {
                assert!(b.get_value().unwrap() == (expected & 1 == 1));
            }
            Boolean::Not(ref b) => {
                assert!(b.get_value().unwrap() != (expected & 1 == 1));
            }
            Boolean::Constant(_) => unreachable!(),
        }

        expected >>= 1;
    }
}

#[test]
fn test_uint64_from_bits() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..1000 {
        let v = (0..64).map(|_| Boolean::constant(rng.gen())).collect::<Vec<_>>();

        let b = UInt64::from_bits_le(&v);

        for (i, bit_gadget) in b.bits.iter().enumerate() {
            match *bit_gadget {
                Boolean::Constant(bit_gadget) => {
                    assert!(bit_gadget == ((b.value.unwrap() >> i) & 1 == 1));
                }
                _ => unreachable!(),
            }
        }

        let expected_to_be_same = b.to_bits_le();

        for x in v.iter().zip(expected_to_be_same.iter()) {
            match x {
                (&Boolean::Constant(true), &Boolean::Constant(true)) => {}
                (&Boolean::Constant(false), &Boolean::Constant(false)) => {}
                _ => unreachable!(),
            }
        }
    }
}

#[test]
fn test_uint64_rotr() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    let mut num = rng.gen();

    let a = UInt64::constant(num);

    for i in 0..64 {
        let b = a.rotr(i);

        assert!(b.value.unwrap() == num);

        let mut tmp = num;
        for b in &b.bits {
            match *b {
                Boolean::Constant(b) => {
                    assert_eq!(b, tmp & 1 == 1);
                }
                _ => unreachable!(),
            }

            tmp >>= 1;
        }

        num = num.rotate_right(1);
    }
}

#[test]
fn test_uint64_xor() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..1000 {
        let mut cs = TestConstraintSystem::<Fr>::new();

        let a: u64 = rng.gen();
        let b: u64 = rng.gen();
        let c: u64 = rng.gen();

        let mut expected = a ^ b ^ c;

        let a_bit = UInt64::alloc(cs.ns(|| "a_bit"), || Ok(a)).unwrap();
        let b_bit = UInt64::constant(b);
        let c_bit = UInt64::alloc(cs.ns(|| "c_bit"), || Ok(c)).unwrap();

        let r = a_bit.xor(cs.ns(|| "first xor"), &b_bit).unwrap();
        let r = r.xor(cs.ns(|| "second xor"), &c_bit).unwrap();

        assert!(cs.is_satisfied());

        assert!(r.value == Some(expected));

        for b in r.bits.iter() {
            match *b {
                Boolean::Is(ref b) => {
                    assert!(b.get_value().unwrap() == (expected & 1 == 1));
                }
                Boolean::Not(ref b) => {
                    assert!(b.get_value().unwrap() != (expected & 1 == 1));
                }
                Boolean::Constant(b) => {
                    assert!(b == (expected & 1 == 1));
                }
            }

            expected >>= 1;
        }
    }
}

#[test]
fn test_uint64_addmany_constants() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..1000 {
        let mut cs = TestConstraintSystem::<Fr>::new();

        let a: u64 = rng.gen();
        let b: u64 = rng.gen();
        let c: u64 = rng.gen();

        let a_bit = UInt64::constant(a);
        let b_bit = UInt64::constant(b);
        let c_bit = UInt64::constant(c);

        let expected = a.wrapping_add(b).wrapping_add(c);

        let r = UInt64::addmany(cs.ns(|| "addition"), &[a_bit, b_bit, c_bit]).unwrap();

        assert!(r.value == Some(expected));

        check_all_constant_bits(expected, r);
    }
}

#[test]
#[allow(clippy::many_single_char_names)]
fn test_uint64_addmany() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..1000 {
        let mut cs = TestConstraintSystem::<Fr>::new();

        let a: u64 = rng.gen();
        let b: u64 = rng.gen();
        let c: u64 = rng.gen();
        let d: u64 = rng.gen();

        let expected = (a ^ b).wrapping_add(c).wrapping_add(d);

        let a_bit = UInt64::alloc(cs.ns(|| "a_bit"), || Ok(a)).unwrap();
        let b_bit = UInt64::constant(b);
        let c_bit = UInt64::constant(c);
        let d_bit = UInt64::alloc(cs.ns(|| "d_bit"), || Ok(d)).unwrap();

        let r = a_bit.xor(cs.ns(|| "xor"), &b_bit).unwrap();
        let r = UInt64::addmany(cs.ns(|| "addition"), &[r, c_bit, d_bit]).unwrap();

        assert!(cs.is_satisfied());

        assert!(r.value == Some(expected));

        check_all_allocated_bits(expected, r);

        // Flip a bit_gadget and see if the addition constraint still works
        if cs.get("addition/result bit_gadget 0/boolean").is_zero() {
            cs.set("addition/result bit_gadget 0/boolean", Fr::one());
        } else {
            cs.set("addition/result bit_gadget 0/boolean", Fr::zero());
        }

        assert!(!cs.is_satisfied());
    }
}

#[test]
fn test_uint64_sub_constants() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..1000 {
        let mut cs = TestConstraintSystem::<Fr>::new();

        let a: u64 = rng.gen_range(u64::MAX / 2u64..u64::MAX);
        let b: u64 = rng.gen_range(0u64..u64::MAX / 2u64);

        let a_bit = UInt64::constant(a);
        let b_bit = UInt64::constant(b);

        let expected = a.wrapping_sub(b);

        let r = a_bit.sub(cs.ns(|| "subtraction"), &b_bit).unwrap();

        assert!(r.value == Some(expected));

        check_all_constant_bits(expected, r);
    }
}

#[test]
fn test_uint64_sub() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..1000 {
        let mut cs = TestConstraintSystem::<Fr>::new();

        let a: u64 = rng.gen_range(u64::MAX / 2u64..u64::MAX);
        let b: u64 = rng.gen_range(0u64..u64::MAX / 2u64);

        let expected = a.wrapping_sub(b);

        let a_bit = UInt64::alloc(cs.ns(|| "a_bit"), || Ok(a)).unwrap();
        let b_bit =
            if b > u64::MAX / 4 { UInt64::constant(b) } else { UInt64::alloc(cs.ns(|| "b_bit"), || Ok(b)).unwrap() };

        let r = a_bit.sub(cs.ns(|| "subtraction"), &b_bit).unwrap();

        assert!(cs.is_satisfied());

        assert!(r.value == Some(expected));

        check_all_allocated_bits(expected, r);

        // Flip a bit_gadget and see if the subtraction constraint still works
        if cs.get("subtraction/add_not/result bit_gadget 0/boolean").is_zero() {
            cs.set("subtraction/add_not/result bit_gadget 0/boolean", Fr::one());
        } else {
            cs.set("subtraction/add_not/result bit_gadget 0/boolean", Fr::zero());
        }

        assert!(!cs.is_satisfied());
    }
}

#[test]
fn test_uint64_mul_constants() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..1000 {
        let mut cs = TestConstraintSystem::<Fr>::new();

        let a: u64 = rng.gen();
        let b: u64 = rng.gen();

        let a_bit = UInt64::constant(a);
        let b_bit = UInt64::constant(b);

        let expected = a.wrapping_mul(b);

        let r = a_bit.mul(cs.ns(|| "multiply"), &b_bit).unwrap();

        assert!(r.value == Some(expected));

        check_all_constant_bits(expected, r);
    }
}

#[test]
fn test_uint64_mul() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..100 {
        let mut cs = TestConstraintSystem::<Fr>::new();

        let a: u64 = rng.gen();
        let b: u64 = rng.gen();

        let expected = a.wrapping_mul(b);

        let a_bit = UInt64::alloc(cs.ns(|| "a_bit"), || Ok(a)).unwrap();
        let b_bit =
            if b > (u64::MAX / 2) { UInt64::constant(b) } else { UInt64::alloc(cs.ns(|| "b_bit"), || Ok(b)).unwrap() };

        let r = a_bit.mul(cs.ns(|| "multiplication"), &b_bit).unwrap();

        assert!(cs.is_satisfied());

        assert!(r.value == Some(expected));

        check_all_allocated_bits(expected, r);

        // Flip a bit_gadget and see if the multiplication constraint still works
        if cs.get("multiplication/partial_products/result bit_gadget 0/boolean").is_zero() {
            cs.set("multiplication/partial_products/result bit_gadget 0/boolean", Fr::one());
        } else {
            cs.set("multiplication/partial_products/result bit_gadget 0/boolean", Fr::zero());
        }

        assert!(!cs.is_satisfied());
    }
}

#[test]
fn test_uint64_div_constants() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..1000 {
        let mut cs = TestConstraintSystem::<Fr>::new();

        let a: u64 = rng.gen();
        let b: u64 = rng.gen();

        let a_bit = UInt64::constant(a);
        let b_bit = UInt64::constant(b);

        let expected = a.wrapping_div(b);

        let r = a_bit.div(cs.ns(|| "division"), &b_bit).unwrap();

        assert!(r.value == Some(expected));

        check_all_constant_bits(expected, r);
    }
}

#[test]
fn test_uint64_div() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..100 {
        let mut cs = TestConstraintSystem::<Fr>::new();

        let a: u64 = rng.gen();
        let b: u64 = rng.gen();

        let expected = a.wrapping_div(b);

        let a_bit = UInt64::alloc(cs.ns(|| "a_bit"), || Ok(a)).unwrap();
        let b_bit =
            if b > u64::MAX / 2 { UInt64::constant(b) } else { UInt64::alloc(cs.ns(|| "b_bit"), || Ok(b)).unwrap() };

        let r = a_bit.div(cs.ns(|| "division"), &b_bit).unwrap();

        assert!(cs.is_satisfied());

        assert!(r.value == Some(expected));

        check_all_allocated_bits(expected, r);

        // Flip a bit_gadget and see if the division constraint still works
        if cs.get("division/r_sub_d_result_0/allocated bit_gadget 0/boolean").is_zero() {
            cs.set("division/r_sub_d_result_0/allocated bit_gadget 0/boolean", Fr::one());
        } else {
            cs.set("division/r_sub_d_result_0/allocated bit_gadget 0/boolean", Fr::zero());
        }

        assert!(!cs.is_satisfied());
    }
}

#[test]
fn test_uint64_pow_constants() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..100 {
        let mut cs = TestConstraintSystem::<Fr>::new();

        let a: u64 = rng.gen_range(0..u64::from(u16::MAX));
        let b: u64 = rng.gen_range(0..4);

        let a_bit = UInt64::constant(a);
        let b_bit = UInt64::constant(b);

        let expected = a.wrapping_pow(b.try_into().unwrap());

        let r = a_bit.pow(cs.ns(|| "exponentiation"), &b_bit).unwrap();

        assert!(r.value == Some(expected));

        check_all_constant_bits(expected, r);
    }
}

#[test]
fn test_uint64_pow() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..4 {
        let mut cs = TestConstraintSystem::<Fr>::new();

        let a: u64 = rng.gen_range(0..u64::from(u16::MAX));
        let b: u64 = rng.gen_range(0..4);

        let expected = a.wrapping_pow(b.try_into().unwrap());

        let a_bit = UInt64::alloc(cs.ns(|| "a_bit"), || Ok(a)).unwrap();
        let b_bit = UInt64::alloc(cs.ns(|| "b_bit"), || Ok(b)).unwrap();

        let r = a_bit.pow(cs.ns(|| "exponentiation"), &b_bit).unwrap();

        assert!(cs.is_satisfied());

        assert!(r.value == Some(expected));

        check_all_allocated_bits(expected, r);

        // Flip a bit_gadget and see if the exponentiation constraint still works
        if cs.get("exponentiation/multiply_by_self_0/partial_products/result bit_gadget 0/boolean").is_zero() {
            cs.set("exponentiation/multiply_by_self_0/partial_products/result bit_gadget 0/boolean", Fr::one());
        } else {
            cs.set("exponentiation/multiply_by_self_0/partial_products/result bit_gadget 0/boolean", Fr::zero());
        }

        assert!(!cs.is_satisfied());
    }
}
