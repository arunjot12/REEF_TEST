//! Tests to make sure that weights and fees match what we
//! expect from Substrate or ORML.
//!
//! These test are not meant to be exhaustive, as it is inevitable that
//! weights in Substrate will change. Instead they are supposed to provide
//! some sort of indicator that calls we consider important (e.g
//! Balances::transfer) have not suddenly changed from under us.

use frame_support::weights::{Weight, constants::WEIGHT_REF_TIME_PER_SECOND};

#[test]
fn sanity_check_weight_per_time_constants_are_as_expected() {
    // Weight is now a 2D struct: { ref_time, proof_size }
    // WEIGHT_PER_SECOND is gone — use WEIGHT_REF_TIME_PER_SECOND instead

    assert_eq!(WEIGHT_REF_TIME_PER_SECOND, 1_000_000_000_000u64);

    assert_eq!(
        WEIGHT_REF_TIME_PER_SECOND / 1_000,
        1_000_000_000u64  // per millis
    );

    assert_eq!(
        WEIGHT_REF_TIME_PER_SECOND / 1_000_000,
        1_000_000u64      // per micros
    );

    assert_eq!(
        WEIGHT_REF_TIME_PER_SECOND / 1_000_000_000,
        1_000u64          // per nanos
    );

    // How to construct Weight values now:
    let one_second = Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND, 0);
    let one_millis = Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND / 1_000, 0);

    assert!(one_second.ref_time() > one_millis.ref_time());
}
