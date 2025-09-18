use revm::{
    context::{Block, BlockEnv, Cfg, CfgEnv, Evm, FrameStack, JournalTr, Transaction, TxEnv},
    database::EmptyDB,
    handler::{instructions::EthInstructions, EthPrecompiles},
    primitives::hardfork::SpecId,
    Database, Journal, MainBuilder, MainContext, MainnetEvm,
};

use crate::context::CustomContext;

impl<BLOCK, TX, CFG, DB, JOURNAL, CHAIN> MainBuilder
    for CustomContext<BLOCK, TX, CFG, DB, JOURNAL, CHAIN>
where
    BLOCK: Block,
    TX: Transaction,
    CFG: Cfg,
    DB: Database,
    JOURNAL: JournalTr<Database = DB>,
{
    type Context = Self;

    fn build_mainnet(self) -> MainnetEvm<Self::Context> {
        Evm {
            ctx: self,
            inspector: (),
            instruction: EthInstructions::default(),
            precompiles: EthPrecompiles::default(),
            frame_stack: FrameStack::new_prealloc(8),
        }
    }

    fn build_mainnet_with_inspector<INSP>(
        self,
        inspector: INSP,
    ) -> MainnetEvm<Self::Context, INSP> {
        Evm {
            ctx: self,
            inspector,
            instruction: EthInstructions::default(),
            precompiles: EthPrecompiles::default(),
            frame_stack: FrameStack::new_prealloc(8),
        }
    }
}

impl MainContext for CustomContext<BlockEnv, TxEnv, CfgEnv, EmptyDB, Journal<EmptyDB>, ()> {
    fn mainnet() -> Self {
        CustomContext::new(EmptyDB::new(), SpecId::default())
    }
}
