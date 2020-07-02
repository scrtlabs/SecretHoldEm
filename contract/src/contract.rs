use cosmwasm_std::{
    generic_err, Api, Env, Extern, HandleResponse, HandleResult, InitResponse, InitResult,
    MigrateResponse, Querier, QueryResponse, QueryResult, StdResult, Storage,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/////////////////////////////// Init ///////////////////////////////
// creates a game and joins as the first player
////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InitMsg {
    CreateTable { seed: u64 },
}

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> InitResult {
    match msg {
        InitMsg::CreateTable { seed } => {
            deps.storage.set(b"creator", &env.message.sender.as_slice());
            deps.storage.set(b"creator_seed", &seed.to_be_bytes());
            Ok(InitResponse::default())
        }
    }
}

/////////////////////////////// Handle ///////////////////////////////
//
//////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Join { seed: u64 },
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> HandleResult {
    match msg {
        HandleMsg::Join { seed } => {
            let player_a = deps.storage.get(b"player_a");
            let player_b = deps.storage.get(b"player_b");

            if player_a.is_some() && player_b.is_some() {
                return Err(generic_err("Table is full."));
            }

            let mut player_name = b"player_b";
            if player_a.is_none() {
                player_name = b"player_a";
            }

            let mut cards_seed = deps
                .storage
                .get(b"creator_seed")
                .expect("No seed from creator.");

            cards_seed.extend(&seed.to_be_bytes());

            let x: &[u8] = Sha256::digest(&cards_seed).as_slice();

            /*
                TODO: figure out deck shuffle logic
            */

            deps.storage
                .set(player_name, &env.message.sender.as_slice());

            Ok(HandleResponse {
                data: None, /* Hand... */
                log: vec![],
                messages: vec![],
            })
        }
    }
}

/////////////////////////////// Query ///////////////////////////////
// These are getters, we only return what's public
// player get their private information as a response to txs (handle)
///////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetAllPublicData {},
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    _deps: &Extern<S, A, Q>,
    _msg: QueryMsg,
) -> QueryResult {
    match _msg {
        QueryMsg::GetAllPublicData {} => {
            return Ok(QueryResponse::default());
        }
    }
}

/////////////////////////////// Migrate ///////////////////////////////
// Isn't supported by the Secret Network, but we must declare this to
// comply with CosmWasm 0.9 API
///////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MigrateMsg {}

pub fn migrate<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    _msg: MigrateMsg,
) -> StdResult<MigrateResponse> {
    Ok(MigrateResponse::default())
}
