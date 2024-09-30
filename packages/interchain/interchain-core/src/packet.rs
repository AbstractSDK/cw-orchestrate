use cosmwasm_std::{Binary, StdError};
use cw_orch_core::environment::CwEnv;
use cw_orch_core::environment::IndexResponse;
use ibc_relayer_types::core::{
    ics04_channel::packet::Sequence,
    ics24_host::identifier::{ChannelId, PortId},
};

use crate::{results::NetworkId, tx::TxId};

/// Structure to hold simple information about a sent packet
#[derive(Debug, Clone)]
pub struct IbcPacketInfo {
    /// Port on which is packet was sent
    pub src_port: PortId,
    /// Channel on which is packet was sent
    pub src_channel: ChannelId,
    /// Packet identification (sequence is `u64` number)
    pub sequence: Sequence,
    /// Chain identification to which the packet was sent
    pub dst_chain_id: NetworkId,
}

/// Raw packet outcome
/// The T generic is used to allow for raw transactions or analyzed transactions to be used
#[derive(Debug, PartialEq, Clone)]
pub enum IbcPacketOutcome<T> {
    /// Packet timeout
    Timeout {
        /// Only a timeout transaction gets broadcasted
        timeout_tx: T,
    },
    /// Packet successfully transferred
    Success {
        /// The packets gets transmitted to the dst chain
        receive_tx: T,
        /// The ack is broadcasted back on the src chain
        ack_tx: T,
        /// The raw binary acknowledgement retrieved from `ack_tx`
        ack: Binary,
    },
}

/// The result of following a single packet across IBC
/// This allows indentifying the different transactions involved as well as the result of the packet transmission
pub struct SinglePacketFlow<Chain: CwEnv> {
    /// The transaction during which the packet was sent
    ///
    /// Can optionally be specified, depending on the environment on which the implementation is done
    /// This is not available for the [`Mock`] implementation for instance
    pub send_tx: Option<TxId<Chain>>,
    /// Outcome transactions of the packet (+ eventual acknowledgment)
    pub outcome: IbcPacketOutcome<TxId<Chain>>,
}

/// The result of following all packets sent across IBC during a single transaction.
///
/// This structure is nested and will also await all packets emitted during the subsequent (receive/acknowledgement/timeout) transactions
#[derive(Clone)]
pub struct NestedPacketsFlow<Chain: CwEnv> {
    /// Identification of the transaction
    pub tx_id: TxId<Chain>,
    /// Result of following a packet + Recursive Analysis of the resulting transactions for additional IBC packets
    pub packets: Vec<IbcPacketOutcome<NestedPacketsFlow<Chain>>>,
}

impl<Chain: CwEnv> IndexResponse for SinglePacketFlow<Chain> {
    fn events(&self) -> Vec<cosmwasm_std::Event> {
        let mut events: Vec<_> = self
            .send_tx
            .as_ref()
            .map(|tx| tx.response.events())
            .unwrap_or_default();
        let other_events = match &self.outcome {
            IbcPacketOutcome::Timeout { timeout_tx } => timeout_tx.events(),
            IbcPacketOutcome::Success {
                receive_tx,
                ack_tx,
                ack: _,
            } => [receive_tx.events(), ack_tx.events()].concat(),
        };
        events.extend(other_events);

        events
    }

    fn event_attr_value(
        &self,
        event_type: &str,
        attr_key: &str,
    ) -> cosmwasm_std::StdResult<String> {
        self.send_tx
            .as_ref()
            .map(|r| r.event_attr_value(event_type, attr_key))
            .and_then(|res| res.ok())
            .or_else(|| match &self.outcome {
                IbcPacketOutcome::Timeout { timeout_tx } => {
                    timeout_tx.event_attr_value(event_type, attr_key).ok()
                }
                IbcPacketOutcome::Success {
                    receive_tx,
                    ack_tx,
                    ack: _,
                } => receive_tx
                    .event_attr_value(event_type, attr_key)
                    .or_else(|_| ack_tx.event_attr_value(event_type, attr_key))
                    .ok(),
            })
            .ok_or(StdError::generic_err(format!(
                "event of type {event_type} does not have a value at key {attr_key}"
            )))
    }

    fn event_attr_values(&self, event_type: &str, attr_key: &str) -> Vec<String> {
        let mut all_results: Vec<_> = self
            .send_tx
            .as_ref()
            .map(|tx| tx.response.event_attr_values(event_type, attr_key))
            .unwrap_or_default();
        let other_results = match &self.outcome {
            IbcPacketOutcome::Timeout { timeout_tx } => {
                timeout_tx.event_attr_values(event_type, attr_key)
            }
            IbcPacketOutcome::Success {
                receive_tx,
                ack_tx,
                ack: _,
            } => [
                receive_tx.event_attr_values(event_type, attr_key),
                ack_tx.event_attr_values(event_type, attr_key),
            ]
            .concat(),
        };
        all_results.extend(other_results);

        all_results
    }

    fn data(&self) -> Option<Binary> {
        unimplemented!("No data fields on Ibc Packet Flow, this is not well defined")
    }
}

impl<Chain: CwEnv> IndexResponse for NestedPacketsFlow<Chain> {
    fn events(&self) -> Vec<cosmwasm_std::Event> {
        let mut self_events = self.tx_id.response.events();
        let other_events = self
            .packets
            .iter()
            .flat_map(|packet_result| match &packet_result {
                IbcPacketOutcome::Timeout { timeout_tx } => timeout_tx.events(),
                IbcPacketOutcome::Success {
                    receive_tx,
                    ack_tx,
                    ack: _,
                } => [receive_tx.events(), ack_tx.events()].concat(),
            });
        self_events.extend(other_events);
        self_events
    }

    fn event_attr_value(
        &self,
        event_type: &str,
        attr_key: &str,
    ) -> cosmwasm_std::StdResult<String> {
        self.tx_id
            .response
            .event_attr_value(event_type, attr_key)
            .or_else(|_| {
                self.packets
                    .iter()
                    .find_map(|packet_result| match &packet_result {
                        IbcPacketOutcome::Timeout { timeout_tx } => {
                            timeout_tx.event_attr_value(event_type, attr_key).ok()
                        }
                        IbcPacketOutcome::Success {
                            receive_tx,
                            ack_tx,
                            ack: _,
                        } => receive_tx
                            .event_attr_value(event_type, attr_key)
                            .or_else(|_| ack_tx.event_attr_value(event_type, attr_key))
                            .ok(),
                    })
                    .ok_or(StdError::generic_err(format!(
                        "event of type {event_type} does not have a value at key {attr_key}"
                    )))
            })
    }

    fn event_attr_values(&self, event_type: &str, attr_key: &str) -> Vec<String> {
        let mut all_results = self.tx_id.response.event_attr_values(event_type, attr_key);

        all_results.extend(self.packets.iter().flat_map(|packet_result| {
            match &packet_result {
                IbcPacketOutcome::Timeout { timeout_tx } => {
                    timeout_tx.event_attr_values(event_type, attr_key)
                }
                IbcPacketOutcome::Success {
                    receive_tx,
                    ack_tx,
                    ack: _,
                } => [
                    receive_tx.event_attr_values(event_type, attr_key),
                    ack_tx.event_attr_values(event_type, attr_key),
                ]
                .concat(),
            }
        }));

        all_results
    }

    fn data(&self) -> Option<Binary> {
        unimplemented!("No data fields on Ibc Tx Analysis")
    }
}

pub mod success {
    use cosmwasm_std::Binary;
    use cw_orch_core::environment::CwEnv;

    use crate::tx::TxId;

    /// Success packet outcome. This is the result of a packet analysis.
    /// The T generic is used to allow for raw transactions or analyzed transactions to be used
    #[derive(Debug, PartialEq, Clone)]
    pub struct SuccessIbcPacketOutcome<T> {
        /// The packets gets transmitted to the dst chain
        pub receive_tx: T,
        /// The ack is broadcasted back on the src chain
        pub ack_tx: T,
        /// The raw binary acknowledgement retrieved from `ack_tx`
        pub ack: Binary,
    }

    /// Success Packet Flow. This is the result of a packet analysis.
    ///
    /// This allows identifying the different transactions involved.
    #[derive(Clone)]
    pub struct SuccessSinglePacketFlow<Chain: CwEnv> {
        /// The transaction during which the packet was sent
        ///
        /// Can optionally be specified, depending on the environment on which the implementation is done
        /// This is not available for the [`Mock`] implementation for instance
        pub send_tx: Option<TxId<Chain>>,
        /// Outcome transactions of the packet (+ eventual acknowledgment)
        pub outcome: SuccessIbcPacketOutcome<TxId<Chain>>,
    }

    /// The result of following all packets sent across IBC during a single transaction.
    ///
    /// This structure is nested and will also await all packets emitted during the subsequent (receive/acknowledgement) transactions
    #[derive(Clone)]
    pub struct SuccessNestedPacketsFlow<Chain: CwEnv> {
        /// Identification of the transaction
        pub tx_id: TxId<Chain>,
        /// Result of following a packet + Recursive Analysis of the resulting transactions for additional IBC packets
        pub packets: Vec<SuccessIbcPacketOutcome<SuccessNestedPacketsFlow<Chain>>>,
    }
}

mod debug {
    use cw_orch_core::environment::CwEnv;

    use super::{
        success::{SuccessNestedPacketsFlow, SuccessSinglePacketFlow},
        NestedPacketsFlow, SinglePacketFlow,
    };

    impl<C: CwEnv> std::fmt::Debug for SinglePacketFlow<C> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("SinglePacketFlow")
                .field("send_tx", &self.send_tx)
                .field("outcome", &self.outcome)
                .finish()
        }
    }

    impl<C: CwEnv> std::fmt::Debug for NestedPacketsFlow<C> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("NestedPacketsFlow")
                .field("tx_id", &self.tx_id)
                .field("packets", &self.packets)
                .finish()
        }
    }

    impl<C: CwEnv> std::fmt::Debug for SuccessSinglePacketFlow<C> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("SuccessSinglePacketFlow")
                .field("sent_tx", &self.send_tx)
                .field("outcome", &self.outcome)
                .finish()
        }
    }

    impl<C: CwEnv> std::fmt::Debug for SuccessNestedPacketsFlow<C> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("SuccessNestedPacketsFlow")
                .field("tx_id", &self.tx_id)
                .field("packets", &self.packets)
                .finish()
        }
    }
}
