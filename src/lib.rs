#![allow(incomplete_features)]
#![feature(maybe_uninit_extra)]
#![feature(specialization)]
#![feature(test)]
#![feature(core_intrinsics)]
#![feature(option_result_unwrap_unchecked)]

#[cfg(feature = "use_snmalloc")]
#[global_allocator]
static ALLOC: snmalloc_rs::SnMalloc = snmalloc_rs::SnMalloc;

#[cfg(feature = "use_mimalloc")]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

pub mod cast;
pub mod checker;
pub mod data;
pub mod ds;
pub mod error;
pub mod func;
pub mod intake;
pub mod turbofan;
pub mod tyck;
pub mod util;
pub mod void;
