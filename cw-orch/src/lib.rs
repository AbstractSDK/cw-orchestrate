#![doc(html_logo_url = "https://raw.githubusercontent.com/AbstractSDK/assets/mainline/logo.svg")]
#![doc = include_str ! (concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]
#![deny(missing_docs)]

// macros
pub use cw_orch_contract_derive::{interface, interface_entry_point};
pub use cw_orch_fns_derive::{ExecuteFns, QueryFns};

/// Re-export anyhow for use in the macros
pub extern crate anyhow;

// prelude
pub mod prelude;

pub use cw_orch_environment::contract;
pub use cw_orch_environment::environment;
pub use cw_orch_mock::{Mock, MockState};

/// Re-export tokio, the async runtime when using daemons.
#[cfg(feature = "daemon")]
pub extern crate tokio;
#[cfg(feature = "daemon")]
pub mod daemon;

mod error;
#[cfg(feature = "osmosis-test-tube")]
pub mod osmosis_test_tube;

#[cfg(feature = "daemon")]
pub mod live_mock;
