#![no_std]
//! # Retroshade SDK.
//!
//! SDK for building and emitting custom events through Mercury's Retroshade
//! Soroban VM fork.
//!

pub use retroshade_sdk_macros::Retroshade;

#[link(wasm_import_module = "x")]
extern "C" {
    #[allow(improper_ctypes)]
    #[link_name = "9"]
    pub fn zephyr_emit(target: i64, event: i64) -> i64;
}
