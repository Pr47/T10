#![allow(incomplete_features)]
#![feature(maybe_uninit_extra)]
#![feature(specialization)]
#![feature(test)]
#![feature(core_intrinsics)]
#![feature(option_result_unwrap_unchecked)]

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
