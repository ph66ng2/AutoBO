# Crypto Module Fix

## Problem
`use rand::Rng` in `src/services/crypto.rs` failed because `rand` was not a Cargo dependency.
`OsRng.fill()` requires the `Rng` trait from the `rand` crate to be in scope.

## Fix
Added `rand = "0.8"` to `Cargo.toml` `[dependencies]`.

Note that `OsRng` itself comes from `aes-gcm` (re-exported from `rand_core`), but the `.fill()` 
method is on the `Rng` trait (from `rand`, which extends `RngCore` from `rand_core`).

No changes needed in `crypto.rs` — the code was correct, just missing the dependency.

## Verification
`cargo check` passed with no new warnings.
