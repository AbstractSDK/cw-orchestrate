mod custom_resp;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Uint128,
};
use serde::Serialize;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
#[cfg_attr(feature = "interface", derive(cw_orch::ExecuteFns))]
pub enum ExecuteMsg<T = String>
where
    T: Serialize,
{
    FirstMessage {},
    #[cfg_attr(feature = "interface", payable)]
    SecondMessage {
        /// test doc-comment
        t: T,
    },
    /// test doc-comment
    ThirdMessage {
        /// test doc-comment
        t: T,
    },
    FourthMessage,
    #[cfg_attr(feature = "interface", payable)]
    FifthMessage,
    SixthMessage(u64, String),
    #[cfg_attr(feature = "interface", payable)]
    SeventhMessage(Uint128, String),
}

#[cw_serde]
#[cfg_attr(feature = "interface", derive(cw_orch::QueryFns))]
#[derive(QueryResponses)]
pub enum QueryMsg<T = String>
where
    T: Serialize,
{
    #[returns(String)]
    /// test-doc-comment
    FirstQuery {},
    #[returns(String)]
    SecondQuery {
        /// test doc-comment
        t: T,
    },
    #[returns(String)]
    ThirdQuery,
    #[returns(u64)]
    FourthQuery(u64, String),
}

#[cw_serde]
pub struct MigrateMsg {
    pub t: String,
}

#[cfg_attr(feature = "export", cosmwasm_std::entry_point)]
#[cfg_attr(feature = "interface", cw_orch::interface_entry_point)]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    Ok(Response::new().add_attribute("action", "instantiate"))
}

#[cfg_attr(feature = "interface", cw_orch::interface_entry_point)]
#[cfg_attr(feature = "export", cosmwasm_std::entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::FirstMessage {} => {
            Ok(Response::new().add_attribute("action", "first message passed"))
        }
        ExecuteMsg::SecondMessage { t: _ } => Err(StdError::generic_err("Second Message Failed")),
        ExecuteMsg::ThirdMessage { .. } => {
            Ok(Response::new().add_attribute("action", "third message passed"))
        }
        ExecuteMsg::FourthMessage => {
            Ok(Response::new().add_attribute("action", "fourth message passed"))
        }
        ExecuteMsg::FifthMessage => {
            if info.funds.is_empty() {
                return Err(StdError::generic_err("Coins missing"));
            }
            Ok(Response::new().add_attribute("action", "fourth message passed"))
        }
        ExecuteMsg::SixthMessage(_, _) => {
            Ok(Response::new().add_attribute("action", "sixth message passed"))
        }
        ExecuteMsg::SeventhMessage(amount, denom) => {
            let c = info.funds[0].clone();
            if c.amount != amount && c.denom.ne(&denom) {
                return Err(StdError::generic_err("Coins don't match message"));
            }
            Ok(Response::new().add_attribute("action", "fourth message passed"))
        }
    }
}

#[cfg_attr(feature = "interface", cw_orch::interface_entry_point)]
#[cfg_attr(feature = "export", cosmwasm_std::entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::FirstQuery {} => to_binary("first query passed"),
        QueryMsg::SecondQuery { .. } => Err(StdError::generic_err("Query not available")),
        QueryMsg::ThirdQuery => to_binary("third query passed"),
        QueryMsg::FourthQuery(_, _) => to_binary(&4u64),
    }
}

#[cfg_attr(feature = "interface", cw_orch::interface_entry_point)]
#[cfg_attr(feature = "export", cosmwasm_std::entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, msg: MigrateMsg) -> StdResult<Response> {
    if msg.t.eq("success") {
        Ok(Response::new())
    } else {
        Err(StdError::generic_err(
            "migrate endpoint reached but no test implementation",
        ))
    }
}

#[cfg(test)]
mod test {
    use super::MockContract as LocalMockContract;
    use super::*;
    use cosmwasm_std::coins;
    use cw_orch::prelude::*;
    #[test]
    fn compiles() -> Result<(), CwOrchError> {
        // We need to check we can still call the execute msgs conveniently
        let sender = Addr::unchecked("sender");
        let mock = Mock::new(&sender);
        mock.set_balance(&sender, coins(156 * 2, "ujuno"))?;
        let contract = LocalMockContract::new("mock-contract", mock.clone());

        contract.upload()?;
        contract.instantiate(&InstantiateMsg {}, None, None)?;
        contract.first_message()?;
        contract.second_message("s".to_string(), &[]).unwrap_err();
        contract.fourth_message().unwrap();
        contract.fifth_message(&coins(156, "ujuno")).unwrap();
        contract.sixth_message(45, "moneys".to_string()).unwrap();

        contract
            .seventh_message(156u128.into(), "ujuno".to_string(), &coins(156, "ujuno"))
            .unwrap();

        contract.first_query().unwrap();
        contract.second_query("arg".to_string()).unwrap_err();
        contract.third_query().unwrap();
        contract.fourth_query(45u64, "moneys".to_string()).unwrap();

        Ok(())
    }
}
