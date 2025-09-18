//! Custom opcodes example
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![allow(dead_code)]
use revm::{
    context::TxEnv, database::EmptyDB, primitives::hardfork::SpecId, ExecuteEvm, MainBuilder,
    MainContext,
};

use crate::context::CustomContext;

mod block;
mod context;
mod mainnet_builder;

/// Demonstrates how to implement and use custom context in REVM.
/// This example shows how to create a custom context whose block is a
/// custom block having an extra field, the sum of all the values in the block.
pub fn main() {
    // EVM execution example
    let ctx: CustomContext = CustomContext::new(EmptyDB::new(), SpecId::default());
    let mut evm = ctx.build_mainnet();
    let _res = evm.transact_one(TxEnv::default());

    // EVM execution with commit
    let ctx = CustomContext::mainnet();
    let mut evm = ctx.build_mainnet();
    let _res = evm.transact_one(TxEnv::default());
}
