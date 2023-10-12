// ANCHOR: full_counter_example
use counter_contract::{
    msg::InstantiateMsg, CounterContract, CounterExecuteMsgFns, CounterQueryMsgFns,
};
use cw_orch::{anyhow, prelude::*, tokio};
use tokio::runtime::Runtime;

const LOCAL_MNEMONIC: &str = "clip hire initial neck maid actor venue client foam budget lock catalog sweet steak waste crater broccoli pipe steak sister coyote moment obvious choose";
pub fn main() -> anyhow::Result<()> {
    std::env::set_var("LOCAL_MNEMONIC", LOCAL_MNEMONIC);
    // ANCHOR: chain_construction
    dotenv::dotenv().ok(); // Used to load the `.env` file if any
    pretty_env_logger::init(); // Used to log contract and chain interactions

    let rt = Runtime::new()?;
    let network = networks::LOCAL_JUNO;
    let chain = DaemonBuilder::default()
        .handle(rt.handle())
        .chain(network)
        .build()?;
    // ANCHOR_END: chain_construction

    // ANCHOR: contract_interaction

    let counter = CounterContract::new("counter_contract", chain);

    // ANCHOR: clean
    counter.upload()?;
    counter.instantiate(&InstantiateMsg { count: 0 }, None, None)?;
    counter.increment()?;

    let count = counter.get_count()?;
    assert_eq!(count.count, 1);
    // ANCHOR_END: clean
    // ANCHOR_END: contract_interaction

    Ok(())
}
// ANCHOR_END: full_counter_example
