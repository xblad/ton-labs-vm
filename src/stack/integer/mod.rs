/*
* Copyright 2018-2020 TON DEV SOLUTIONS LTD.
*
* Licensed under the SOFTWARE EVALUATION License (the "License"); you may not use
* this file except in compliance with the License.  You may obtain a copy of the
* License at: https://ton.dev/licenses
*
* Unless required by applicable law or agreed to in writing, software
* distributed under the License is distributed on an "AS IS" BASIS,
* WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
* See the License for the specific TON DEV software governing permissions and
* limitations under the License.
*/

#[macro_use]
pub mod behavior;
mod fmt;

use self::utils::*;
pub use self::fmt::*;
use std::cmp;
use std::cmp::Ordering;
use stack::integer::behavior::OperationBehavior;

use num::{bigint::Sign, Zero, Signed, BigUint};
use types::{
    Result,
    ResultOpt
};
use core::mem;
use num_traits::One;

type Int = num::BigInt;

#[derive(Clone, Debug, PartialEq, Eq)]
enum IntegerValue {
    NaN,
    Value(Int)
}

impl cmp::PartialOrd for IntegerValue {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        match (self, other) {
            (IntegerValue::Value(x), IntegerValue::Value(y)) => {
                x.partial_cmp(y)
            },
            _ => None
        }
    }
}

impl IntegerValue {
    #[inline]
    pub fn unwrap(&self) -> &Int {
        match self {
            IntegerValue::Value(ref x) => x,
            _ => panic!("Not a number!")
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntegerData {
    value: IntegerValue
}

impl IntegerData {
    /// Constructs new (set to 0) value. This is just a wrapper for Self::zero().
    #[inline]
    pub fn new() -> IntegerData {
        Self::zero()
    }

    /// Constructs new (set to 0) value.
    #[inline]
    pub fn zero() -> IntegerData {
        IntegerData {
            value: IntegerValue::Value(Int::zero())
        }
    }

    /// Constructs new (set to 1) value.
    #[inline]
    pub fn one() -> IntegerData {
        IntegerData {
            value: IntegerValue::Value(Int::one())
        }
    }

    /// Constructs new (set to -1) value.
    #[inline]
    pub fn minus_one() -> IntegerData {
        IntegerData {
            value: IntegerValue::Value(
                Int::from_biguint(Sign::Minus, BigUint::one())
            )
        }
    }

    /// Constructs new Not-a-Number (NaN) value.
    #[inline]
    pub fn nan() -> IntegerData {
        IntegerData {
            value: IntegerValue::NaN
        }
    }

    /// Clears value (sets to 0).
    #[inline]
    pub fn withdraw(&mut self) -> IntegerData {
        mem::replace(self, IntegerData::new())
    }

    /// Replaces value to a given one.
    #[inline]
    pub fn replace(&mut self, new_value: IntegerData) {
        mem::replace(self, new_value);
    }

    /// Checks if value is a Not-a-Number (NaN).
    #[inline]
    pub fn is_nan(&self) -> bool {
        self.value == IntegerValue::NaN
    }

    /// Checks if value is negative (less than zero).
    #[inline]
    pub fn is_neg(&self) -> bool {
        match &self.value {
            IntegerValue::NaN => false,
            IntegerValue::Value(ref value) => value.is_negative()
        }
    }

    /// Checks if value is zero.
    #[inline]
    pub fn is_zero(&self) -> bool {
        match &self.value {
            IntegerValue::NaN => false,
            IntegerValue::Value(ref value) => value.is_zero()
        }
    }

    /// Compares value with another taking in account behavior of operation.
    #[inline]
    pub fn cmp<T: OperationBehavior>(&self, other: &IntegerData) -> ResultOpt<Ordering> {
        if self.is_nan() || other.is_nan() {
            on_nan_parameter!(T)?;
            return Ok(None);
        }

        Ok(Some(self.value.unwrap().cmp(other.value.unwrap())))
    }

    /// Returns true if signed value fits into a given bits size; otherwise false.
    #[inline]
    pub fn fits_in(&self, bits: usize) -> bool {
        self.bitsize() <= bits
    }

    /// Returns true if unsigned value fits into a given bits size; otherwise false.
    #[inline]
    pub fn ufits_in(&self, bits: usize) -> bool {
        !self.is_neg() && self.ubitsize() <= bits
    }

    /// Determines a fewest bits necessary to express signed value.
    #[inline]
    pub fn bitsize(&self) -> usize {
        process_value(&self, |value| {
            bitsize(value)
        })
    }

    /// Determines a fewest bits necessary to express unsigned value.
    #[inline]
    pub fn ubitsize(&self) -> usize {
        process_value(&self, |value| {
            debug_assert!(!value.is_negative());
            value.bits()
        })
    }
}

impl AsRef<IntegerData> for IntegerData {
    #[inline]
    fn as_ref(&self) -> &IntegerData {
        self
    }
}

#[macro_use]
pub mod utils {
    use super::*;
    use std::ops::Not;

    #[inline]
    pub fn process_value<F, R>(value: &IntegerData, call_on_valid: F) -> R
    where
        F: Fn(&Int) -> R,
    {
        match value.value {
            IntegerValue::NaN => panic!("IntegerData must be a valid number"),
            IntegerValue::Value(ref value) => call_on_valid(value),
        }
    }

    /// This macro extracts internal Int value from IntegerData using given NaN behavior
    /// and NaN constructor.
    macro_rules! extract_value {
        ($T: ident, $v: ident, $nan_constructor: ident) => {
            match $v.value {
                IntegerValue::NaN => {
                    on_nan_parameter!($T)?;
                    return Ok($nan_constructor());
                },
                IntegerValue::Value(ref $v) => $v,
            }
        }
    }

    /// Unary operation. Checks lhs for NaN, unwraps it, calls closure and returns wrapped result.
    #[inline]
    pub fn unary_op<T, F, FNaN, FRes, RInt, R>(
        lhs: &IntegerData,
        callback: F,
        nan_constructor: FNaN,
        result_processor: FRes
    ) -> Result<R>
    where
        T: behavior::OperationBehavior,
        F: Fn(&Int) -> RInt,
        FNaN: Fn() -> R,
        FRes: Fn(RInt, FNaN) -> Result<R>,
    {
        let lhs = extract_value!(T, lhs, nan_constructor);

        result_processor(callback(lhs), nan_constructor)
    }

    /// Binary operation. Checks lhs & rhs for NaN, unwraps them, calls closure and returns wrapped result.
    #[inline]
    pub fn binary_op<T, F, FNaN, FRes, RInt, R>(
        lhs: &IntegerData,
        rhs: &IntegerData,
        callback: F,
        nan_constructor: FNaN,
        result_processor: FRes
    ) -> Result<R>
    where
        T: behavior::OperationBehavior,
        F: Fn(&Int, &Int) -> RInt,
        FNaN: Fn() -> R,
        FRes: Fn(RInt, FNaN) -> Result<R>,
    {
        let lhs = extract_value!(T, lhs, nan_constructor);
        let rhs = extract_value!(T, rhs, nan_constructor);

        result_processor(callback(lhs, rhs), nan_constructor)
    }

    #[inline]
    pub fn process_single_result<T, FNaN>(result: Int, nan_constructor: FNaN) -> Result<IntegerData>
    where
        T: behavior::OperationBehavior,
        FNaN: Fn() -> IntegerData,
    {
        IntegerData::from(result).or_else(|_| {
            on_integer_overflow!(T)?;
            Ok(nan_constructor())
        })
    }

    #[inline]
    pub fn process_double_result<T, FNaN>(result: (Int, Int), nan_constructor: FNaN)
        -> Result<(IntegerData, IntegerData)>
    where
        T: behavior::OperationBehavior,
        FNaN: Fn() -> (IntegerData, IntegerData),
    {
        let (r1, r2) = result;
        match IntegerData::from(r1) {
            Ok(r1) => Ok((r1, IntegerData::from(r2).unwrap())),
            Err(_) => {
                on_integer_overflow!(T)?;
                Ok(nan_constructor())
            },
        }
    }

    #[inline]
    pub fn construct_single_nan() -> IntegerData {
        IntegerData::nan()
    }

    #[inline]
    pub fn construct_double_nan() -> (IntegerData, IntegerData) {
        (construct_single_nan(), construct_single_nan())
    }

    /// Integer overflow checking. Returns true, if value fits into IntegerData; otherwise false.
    #[inline]
    pub fn check_overflow(value: &Int) -> bool {
        bitsize(value) < 258
    }

    #[inline]
    pub fn bitsize(value: &Int) -> usize {
        if value.is_zero() || *value == Int::from_biguint(Sign::Minus, BigUint::one()) {
            return 1;
        }
        let res = value.bits();
        if value.is_positive() {
            return res + 1;
        }
        // For negative values value.bits() returns correct result only when value is power of 2.
        let mut modpow2 = value.abs();
        modpow2 &= &modpow2 - 1;
        if modpow2.is_zero() {
            return res;
        }
        res + 1
    }

    /// Perform in-place two's complement of the given digit iterator
    /// starting from the least significant byte.
    #[inline]
    pub fn twos_complement<'a, I>(digits: I)
    where
        I: IntoIterator<Item = &'a mut u32>,
    {
        let mut carry = true;
        for d in digits {
            *d = d.not();
            if carry {
                *d = d.wrapping_add(1);
                carry = d.is_zero();
            }
        }
    }
}

#[macro_use]
pub mod conversion;
pub mod serialization;
pub mod math;
pub mod bitlogics;
pub mod traits;

