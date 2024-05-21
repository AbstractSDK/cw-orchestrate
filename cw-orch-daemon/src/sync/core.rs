use std::{fmt::Debug, sync::Arc};

use super::super::senders::base_sender::Wallet;
use crate::{
    queriers::{Bank, CosmWasm, Node},
    CosmTxResponse, DaemonAsyncBase, DaemonBuilder, DaemonError, DaemonState,
};
use cosmwasm_std::{Addr, Coin};
use cw_orch_core::{
    contract::{interface_traits::Uploadable, WasmPath},
    environment::{ChainState, DefaultQueriers, QueryHandler, TxHandler},
};
use cw_orch_traits::stargate::Stargate;
use serde::Serialize;
use tokio::runtime::Handle;
use tonic::transport::Channel;

use crate::senders::sender_trait::SenderTrait;

#[derive(Clone)]
/**
    Represents a blockchain node.
    Is constructed with the [DaemonBuilder].

    ## Usage

    ```rust,no_run
    use cw_orch_daemon::{Daemon, networks};
    use tokio::runtime::Runtime;

    let rt = Runtime::new().unwrap();
    let daemon: Daemon = Daemon::builder()
        .chain(networks::JUNO_1)
        .build()
        .unwrap();
    ```
    ## Environment Execution

    The Daemon implements [`TxHandler`] which allows you to perform transactions on the chain.

    ## Querying

    Different Cosmos SDK modules can be queried through the daemon by calling the [`Daemon.query_client<Querier>`] method with a specific querier.
    See [Querier](crate::queriers) for examples.
*/
pub struct DaemonBase<SenderGen: SenderTrait> {
    pub daemon: DaemonAsyncBase<SenderGen>,
    /// Runtime handle to execute async tasks
    pub rt_handle: Handle,
}

pub type Daemon = DaemonBase<Wallet>;

impl<SenderGen: SenderTrait> DaemonBase<SenderGen> {
    /// Get the daemon builder
    pub fn builder() -> DaemonBuilder {
        DaemonBuilder::default()
    }

    /// Get the channel configured for this Daemon
    pub fn channel(&self) -> Channel {
        self.daemon.state.grpc_channel.clone()
    }

    /// Get the channel configured for this Daemon
    pub fn wallet(&self) -> SenderGen {
        self.daemon.sender.clone()
    }

    // /// Returns a new [`DaemonBuilder`] with the current configuration.
    // /// Does not consume the original [`Daemon`].
    // pub fn rebuild(&self) -> DaemonBuilderBase<SenderGen> {
    //     let mut builder = Self::builder();
    //     builder
    //         .chain(self.state().chain_data.clone())
    //         .sender((*self.daemon.sender).clone())
    //         .deployment_id(&self.state().deployment_id);
    //     builder
    // }
}

impl<SenderGen: SenderTrait> ChainState for DaemonBase<SenderGen> {
    type Out = Arc<DaemonState>;

    fn state(&self) -> Self::Out {
        self.daemon.state.clone()
    }
}

// Execute on the real chain, returns tx response
impl<SenderGen: SenderTrait> TxHandler for DaemonBase<SenderGen> {
    type Response = CosmTxResponse;
    type Error = DaemonError;
    type ContractSource = WasmPath;
    type Sender = SenderGen;

    fn sender(&self) -> Addr {
        self.daemon.sender.address().unwrap()
    }

    fn set_sender(&mut self, sender: Self::Sender) {
        self.daemon.sender = sender
    }

    fn upload<T: Uploadable>(&self, uploadable: &T) -> Result<Self::Response, DaemonError> {
        self.rt_handle.block_on(self.daemon.upload(uploadable))
    }

    fn execute<E: Serialize>(
        &self,
        exec_msg: &E,
        coins: &[cosmwasm_std::Coin],
        contract_address: &Addr,
    ) -> Result<Self::Response, DaemonError> {
        self.rt_handle
            .block_on(self.daemon.execute(exec_msg, coins, contract_address))
    }

    fn instantiate<I: Serialize + Debug>(
        &self,
        code_id: u64,
        init_msg: &I,
        label: Option<&str>,
        admin: Option<&Addr>,
        coins: &[Coin],
    ) -> Result<Self::Response, DaemonError> {
        self.rt_handle.block_on(
            self.daemon
                .instantiate(code_id, init_msg, label, admin, coins),
        )
    }

    fn migrate<M: Serialize + Debug>(
        &self,
        migrate_msg: &M,
        new_code_id: u64,
        contract_address: &Addr,
    ) -> Result<Self::Response, DaemonError> {
        self.rt_handle.block_on(
            self.daemon
                .migrate(migrate_msg, new_code_id, contract_address),
        )
    }

    fn instantiate2<I: Serialize + Debug>(
        &self,
        code_id: u64,
        init_msg: &I,
        label: Option<&str>,
        admin: Option<&Addr>,
        coins: &[cosmwasm_std::Coin],
        salt: cosmwasm_std::Binary,
    ) -> Result<Self::Response, Self::Error> {
        self.rt_handle.block_on(
            self.daemon
                .instantiate2(code_id, init_msg, label, admin, coins, salt),
        )
    }
}

impl<SenderGen: SenderTrait> Stargate for DaemonBase<SenderGen> {
    fn commit_any<R>(
        &self,
        msgs: Vec<prost_types::Any>,
        memo: Option<&str>,
    ) -> Result<Self::Response, Self::Error> {
        self.rt_handle
            .block_on(
                self.wallet().commit_tx_any(
                    msgs.iter()
                        .map(|msg| cosmrs::Any {
                            type_url: msg.type_url.clone(),
                            value: msg.value.clone(),
                        })
                        .collect(),
                    memo,
                ),
            )
            .map_err(Into::into)
    }
}

impl<SenderGen: SenderTrait> QueryHandler for DaemonBase<SenderGen> {
    type Error = DaemonError;

    fn wait_blocks(&self, amount: u64) -> Result<(), DaemonError> {
        self.rt_handle.block_on(self.daemon.wait_blocks(amount))?;

        Ok(())
    }

    fn wait_seconds(&self, secs: u64) -> Result<(), DaemonError> {
        self.rt_handle.block_on(self.daemon.wait_seconds(secs))?;

        Ok(())
    }

    fn next_block(&self) -> Result<(), DaemonError> {
        self.rt_handle.block_on(self.daemon.next_block())?;

        Ok(())
    }
}

impl<SenderGen: SenderTrait> DefaultQueriers for DaemonBase<SenderGen> {
    type Bank = Bank;
    type Wasm = CosmWasm<SenderGen>;
    type Node = Node;
}
