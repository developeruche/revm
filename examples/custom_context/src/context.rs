use revm::{
    context::{
        host::LoadError, journaled_state::AccountInfoLoad, Block, Cfg, CfgEnv, ContextError,
        ContextSetters, ContextTr, Host, JournalTr, LocalContext, LocalContextTr, Transaction,
        TransactionType, TxEnv,
    },
    database::EmptyDB,
    interpreter::{SStoreResult, SelfDestructResult, StateLoad},
    primitives::{hardfork::SpecId, Address, Log, StorageKey, StorageValue, B256, U256},
    Database, Journal,
};

use crate::block::CustomBlockEnv;

pub(crate) struct CustomContext<
    BLOCK = CustomBlockEnv,
    TX = TxEnv,
    CFG = CfgEnv,
    DB: Database = EmptyDB,
    JOURNAL: JournalTr<Database = DB> = Journal<DB>,
    CHAIN = (),
    LOCAL: LocalContextTr = LocalContext,
> {
    /// Block information.
    pub block: BLOCK,
    /// Transaction information.
    pub tx: TX,
    /// Configurations.
    pub cfg: CFG,
    /// EVM State with journaling support and database.
    pub journaled_state: JOURNAL,
    /// Inner context.
    pub chain: CHAIN,
    /// Local context that is filled by execution.
    pub local: LOCAL,
    /// Error that happened during execution.
    pub error: Result<(), ContextError<DB::Error>>,
}

impl<
        BLOCK: Block,
        TX: Transaction,
        DB: Database,
        CFG: Cfg,
        JOURNAL: JournalTr<Database = DB>,
        CHAIN,
        LOCAL: LocalContextTr,
    > ContextTr for CustomContext<BLOCK, TX, CFG, DB, JOURNAL, CHAIN, LOCAL>
{
    type Block = BLOCK;
    type Tx = TX;
    type Cfg = CFG;
    type Db = DB;
    type Journal = JOURNAL;
    type Chain = CHAIN;
    type Local = LOCAL;

    #[inline]
    fn tx(&self) -> &Self::Tx {
        &self.tx
    }

    #[inline]
    fn block(&self) -> &Self::Block {
        &self.block
    }

    #[inline]
    fn cfg(&self) -> &Self::Cfg {
        &self.cfg
    }

    #[inline]
    fn journal(&self) -> &Self::Journal {
        &self.journaled_state
    }

    #[inline]
    fn journal_mut(&mut self) -> &mut Self::Journal {
        &mut self.journaled_state
    }

    #[inline]
    fn journal_ref(&self) -> &Self::Journal {
        &self.journaled_state
    }

    #[inline]
    fn db(&self) -> &Self::Db {
        self.journaled_state.db()
    }

    #[inline]
    fn db_mut(&mut self) -> &mut Self::Db {
        self.journaled_state.db_mut()
    }

    #[inline]
    fn chain(&self) -> &Self::Chain {
        &self.chain
    }

    #[inline]
    fn chain_mut(&mut self) -> &mut Self::Chain {
        &mut self.chain
    }

    #[inline]
    fn local(&self) -> &Self::Local {
        &self.local
    }

    #[inline]
    fn local_mut(&mut self) -> &mut Self::Local {
        &mut self.local
    }

    #[inline]
    fn error(&mut self) -> &mut Result<(), ContextError<<Self::Db as Database>::Error>> {
        &mut self.error
    }

    #[inline]
    fn tx_journal_mut(&mut self) -> (&Self::Tx, &mut Self::Journal) {
        (&self.tx, &mut self.journaled_state)
    }

    #[inline]
    fn tx_local_mut(&mut self) -> (&Self::Tx, &mut Self::Local) {
        (&self.tx, &mut self.local)
    }
}

impl<
        BLOCK: Block,
        TX: Transaction,
        DB: Database,
        CFG: Cfg,
        JOURNAL: JournalTr<Database = DB>,
        CHAIN,
        LOCAL: LocalContextTr,
    > ContextSetters for CustomContext<BLOCK, TX, CFG, DB, JOURNAL, CHAIN, LOCAL>
{
    fn set_tx(&mut self, tx: Self::Tx) {
        self.tx = tx;
    }

    fn set_block(&mut self, block: Self::Block) {
        self.block = block;
    }
}

impl<
        BLOCK: Block + Default,
        TX: Transaction + Default,
        DB: Database,
        JOURNAL: JournalTr<Database = DB>,
        CHAIN: Default,
        LOCAL: LocalContextTr + Default,
    > CustomContext<BLOCK, TX, CfgEnv, DB, JOURNAL, CHAIN, LOCAL>
{
    /// Creates a new context with a new database type.
    ///
    /// This will create a new [`Journal`] object.
    pub(crate) fn new(db: DB, spec: SpecId) -> Self {
        let mut journaled_state = JOURNAL::new(db);
        journaled_state.set_spec_id(spec);
        Self {
            tx: TX::default(),
            block: BLOCK::default(),
            cfg: CfgEnv::new_with_spec(spec),
            local: LOCAL::default(),
            journaled_state,
            chain: Default::default(),
            error: Ok(()),
        }
    }
}

impl<
        BLOCK: Block,
        TX: Transaction,
        CFG: Cfg,
        DB: Database,
        JOURNAL: JournalTr<Database = DB>,
        CHAIN,
        LOCAL: LocalContextTr,
    > Host for CustomContext<BLOCK, TX, CFG, DB, JOURNAL, CHAIN, LOCAL>
{
    /* Block */

    fn basefee(&self) -> U256 {
        U256::from(self.block().basefee())
    }

    fn blob_gasprice(&self) -> U256 {
        U256::from(self.block().blob_gasprice().unwrap_or(0))
    }

    fn gas_limit(&self) -> U256 {
        U256::from(self.block().gas_limit())
    }

    fn difficulty(&self) -> U256 {
        self.block().difficulty()
    }

    fn prevrandao(&self) -> Option<U256> {
        self.block().prevrandao().map(|r| r.into())
    }

    fn block_number(&self) -> U256 {
        self.block().number()
    }

    fn timestamp(&self) -> U256 {
        U256::from(self.block().timestamp())
    }

    fn beneficiary(&self) -> Address {
        self.block().beneficiary()
    }

    fn chain_id(&self) -> U256 {
        U256::from(self.cfg().chain_id())
    }

    /* Transaction */

    fn effective_gas_price(&self) -> U256 {
        let basefee = self.block().basefee();
        U256::from(self.tx().effective_gas_price(basefee as u128))
    }

    fn caller(&self) -> Address {
        self.tx().caller()
    }

    fn blob_hash(&self, number: usize) -> Option<U256> {
        let tx = &self.tx();
        if tx.tx_type() != TransactionType::Eip4844 {
            return None;
        }
        tx.blob_versioned_hashes()
            .get(number)
            .map(|t| U256::from_be_bytes(t.0))
    }

    /* Config */

    fn max_initcode_size(&self) -> usize {
        self.cfg().max_initcode_size()
    }

    /* Database */

    fn block_hash(&mut self, requested_number: u64) -> Option<B256> {
        self.db_mut()
            .block_hash(requested_number)
            .map_err(|e| {
                *self.error() = Err(e.into());
            })
            .ok()
    }

    /* Journal */

    /// Gets the transient storage value of `address` at `index`.
    fn tload(&mut self, address: Address, index: StorageKey) -> StorageValue {
        self.journal_mut().tload(address, index)
    }

    /// Sets the transient storage value of `address` at `index`.
    fn tstore(&mut self, address: Address, index: StorageKey, value: StorageValue) {
        self.journal_mut().tstore(address, index, value)
    }

    /// Emits a log owned by `address` with given `LogData`.
    fn log(&mut self, log: Log) {
        self.journal_mut().log(log);
    }

    /// Marks `address` to be deleted, with funds transferred to `target`.
    fn selfdestruct(
        &mut self,
        address: Address,
        target: Address,
    ) -> Option<StateLoad<SelfDestructResult>> {
        self.journal_mut()
            .selfdestruct(address, target)
            .map_err(|e| {
                *self.error() = Err(e.into());
            })
            .ok()
    }

    fn sstore_skip_cold_load(
        &mut self,
        address: Address,
        key: StorageKey,
        value: StorageValue,
        skip_cold_load: bool,
    ) -> Result<StateLoad<SStoreResult>, LoadError> {
        self.journal_mut()
            .sstore_skip_cold_load(address, key, value, skip_cold_load)
            .map_err(|e| {
                let (ret, err) = e.into_parts();
                if let Some(err) = err {
                    *self.error() = Err(err.into());
                }
                ret
            })
    }

    fn sload_skip_cold_load(
        &mut self,
        address: Address,
        key: StorageKey,
        skip_cold_load: bool,
    ) -> Result<StateLoad<StorageValue>, LoadError> {
        self.journal_mut()
            .sload_skip_cold_load(address, key, skip_cold_load)
            .map_err(|e| {
                let (ret, err) = e.into_parts();
                if let Some(err) = err {
                    *self.error() = Err(err.into());
                }
                ret
            })
    }

    fn load_account_info_skip_cold_load(
        &mut self,
        address: Address,
        load_code: bool,
        skip_cold_load: bool,
    ) -> Result<AccountInfoLoad<'_>, LoadError> {
        let error = &mut self.error;
        let journal = &mut self.journaled_state;
        match journal.load_account_info_skip_cold_load(address, load_code, skip_cold_load) {
            Ok(a) => Ok(a),
            Err(e) => {
                let (ret, err) = e.into_parts();
                if let Some(err) = err {
                    *error = Err(err.into());
                }
                Err(ret)
            }
        }
    }
}
