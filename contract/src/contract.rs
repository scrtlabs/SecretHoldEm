use bincode;
use cosmwasm_std::{
    generic_err, Api, Binary, CanonicalAddr, Env, Extern, HandleResponse, HandleResult, HumanAddr,
    InitResponse, InitResult, MigrateResponse, Querier, QueryResponse, QueryResult, StdResult,
    Storage,
};
use rand::{seq::SliceRandom, SeedableRng};
use rand_chacha::ChaChaRng;
use rs_poker::core::{Card, Deck, Hand};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/////////////////////////////// Init ///////////////////////////////
//
////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InitMsg {}

pub fn init<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    _msg: InitMsg,
) -> InitResult {
    Ok(InitResponse::default())
}

/////////////////////////////// Handle ///////////////////////////////
//
//////////////////////////////////////////////////////////////////////
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Table {
    player_a: HumanAddr,
    player_a_wallet: i64,
    player_a_bet: i64,

    player_b: HumanAddr,
    player_b_wallet: i64,
    player_b_bet: i64,

    starter: HumanAddr,
    turn: HumanAddr, // round ends if after a bet: starter != turn && player_a_bet == player_b_bet

    stage: Stage,

    cards: Vec<Card>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
enum Stage {
    PreFlop,
    Flop,
    Turn,
    River,
    Ended { is_draw: bool, winner: HumanAddr },
}

const MAX_CREDIT: i64 = 1_000_000;

// indexes of cards in the deck
const PLAYER_A_FIRST_CARD: usize = 0;
const PLAYER_B_FIRST_CARD: usize = 1;
const PLAYER_A_SECOND_CARD: usize = 2;
const PLAYER_B_SECOND_CARD: usize = 3;
// Pre-flop betting round - burn index 4
const FLOP_FIRST_CARD: usize = 5;
const FLOP_SECOND_CARD: usize = 6;
const FLOP_THIRD_CARD: usize = 7;
// Flop betting round - burn index 8
const TURN_CARD: usize = 9;
// Turn betting round - burn index 10
const RIVER_CARD: usize = 11;
// River betting round

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Join { secret: u64 },
    Raise { amount: i64 },
    Call {},
    Fold {},
    Check {},
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> HandleResult {
    match msg {
        HandleMsg::Join { secret } => {
            let player_a = deps.storage.get(b"player_a");
            let player_b = deps.storage.get(b"player_b");

            if player_a.is_some() && player_b.is_some() {
                return Err(generic_err("Table is full."));
            }

            let player_name = env.message.sender.as_slice();
            let player_secret = &secret.to_be_bytes();

            if player_a.is_none() {
                // player a - just store
                deps.storage.set(b"player_a", player_name);
                deps.storage.set(b"player_a_secret", player_secret);
                return Ok(HandleResponse::default());
            }

            // player b - we can now shuffle the deck

            deps.storage.set(b"player_b", player_name);
            deps.storage.set(b"player_b_secret", player_secret);

            let player_a_secret = deps.storage.get(b"player_a_secret").unwrap();

            let mut combined_secret = player_a_secret.clone();
            combined_secret.extend(player_secret);
            let seed: [u8; 32] = Sha256::digest(&combined_secret).into();

            let mut rng = ChaChaRng::from_seed(seed);
            let mut deck: Vec<Card> = Deck::default().into_iter().collect();
            deck.shuffle(&mut rng);

            let deck_bytes = bincode::serialize(&deck).unwrap();
            deps.storage.set(b"deck", &deck_bytes);

            let a_human_addr = deps
                .api
                .human_address(&CanonicalAddr(Binary(player_a.expect("Error"))))
                .unwrap();
            let b_human_addr = deps
                .api
                .human_address(&CanonicalAddr(Binary(player_name.to_vec())))
                .unwrap();

            let table = Table {
                player_a: a_human_addr.clone(),
                player_b: b_human_addr.clone(),

                player_a_wallet: MAX_CREDIT,
                player_b_wallet: MAX_CREDIT,

                player_a_bet: 0,
                player_b_bet: 0,

                stage: Stage::PreFlop,
                starter: a_human_addr.clone(),
                turn: a_human_addr.clone(),

                cards: vec![],
            };

            let table_bytes = bincode::serialize(&table).unwrap();
            deps.storage.set(b"table", &table_bytes);

            return Ok(HandleResponse::default());
        }
        HandleMsg::Raise { amount } => {
            let mut table: Table =
                bincode::deserialize(&deps.storage.get(b"table").unwrap()).unwrap();
            match table.stage {
                Stage::Ended {
                    is_draw: _,
                    winner: _,
                } => return Err(generic_err("The game is over.")),
                _ => { /* continue */ }
            };

            let me = deps.api.human_address(&env.message.sender).unwrap();

            if me != table.player_a && me != table.player_b {
                return Err(generic_err("You are not a player, go away!"));
            }

            if me != table.turn {
                return Err(generic_err("It's not your turn."));
            }

            if me == table.player_a {
                // I'm player A
                table.player_a_bet = table.player_b_bet + amount;
                table.player_a_wallet = MAX_CREDIT - table.player_a_bet;
                if table.player_a_wallet < 0 {
                    return Err(generic_err(
                        "You don't have enough credits to Raise by that much.",
                    ));
                }

                table.turn = table.player_b.clone();
            } else {
                // I'm player B
                table.player_b_bet = table.player_a_bet + amount;
                table.player_b_wallet = MAX_CREDIT - table.player_b_bet;
                if table.player_b_wallet < 0 {
                    return Err(generic_err(
                        "You don't have enough credits to Raise by that much.",
                    ));
                }

                table.turn = table.player_a.clone();
            }

            let table_bytes = bincode::serialize(&table).unwrap();
            deps.storage.set(b"table", &table_bytes);

            return Ok(HandleResponse::default());
        }
        HandleMsg::Call {} => {
            let mut table: Table =
                bincode::deserialize(&deps.storage.get(b"table").unwrap()).unwrap();
            match table.stage {
                Stage::Ended {
                    is_draw: _,
                    winner: _,
                } => return Err(generic_err("The game is over.")),
                _ => { /* continue */ }
            };

            let me = deps.api.human_address(&env.message.sender).unwrap();

            if me != table.player_a && me != table.player_b {
                return Err(generic_err("You are not a player, go away!"));
            }

            if me != table.turn {
                return Err(generic_err("It's not your turn."));
            }

            if me == table.player_a {
                // I'm player A
                if table.player_a_bet >= table.player_b_bet {
                    return Err(generic_err(
                        "You cannot Call, your bet is bigger or equals to the other player's bet.",
                    ));
                }
                table.player_a_bet = table.player_b_bet;
                table.player_a_wallet = MAX_CREDIT - table.player_a_bet;
                if table.player_a_wallet < 0 {
                    table.player_a_wallet = 0;
                    table.player_a_bet = MAX_CREDIT;
                }

                table.turn = table.player_b.clone();
            } else {
                // I'm player B
                if table.player_b_bet >= table.player_a_bet {
                    return Err(generic_err(
                        "You cannot Call, your bet is bigger or equals to the other player's bet.",
                    ));
                }
                table.player_b_bet = table.player_a_bet;
                table.player_b_wallet = MAX_CREDIT - table.player_b_bet;
                if table.player_b_wallet < 0 {
                    table.player_b_wallet = 0;
                    table.player_b_bet = MAX_CREDIT;
                }
                table.turn = table.player_a.clone();
            }

            if table.turn == table.starter {
                // go to next stage
                table.stage = next(table.stage);
            }

            let table_bytes = bincode::serialize(&table).unwrap();
            deps.storage.set(b"table", &table_bytes);

            return Ok(HandleResponse::default());
        }
        HandleMsg::Fold {} => {
            let mut table: Table =
                bincode::deserialize(&deps.storage.get(b"table").unwrap()).unwrap();
            match table.stage {
                Stage::Ended {
                    is_draw: _,
                    winner: _,
                } => return Err(generic_err("The game is over.")),
                _ => { /* continue */ }
            };

            let me = deps.api.human_address(&env.message.sender).unwrap();

            if me != table.player_a && me != table.player_b {
                return Err(generic_err("You are not a player, go away!"));
            }

            if me != table.turn {
                return Err(generic_err("It's not your turn."));
            }

            if me == table.player_a {
                table.stage = Stage::Ended {
                    is_draw: false,
                    winner: table.player_b.clone(),
                }
            } else {
                table.stage = Stage::Ended {
                    is_draw: false,
                    winner: table.player_a.clone(),
                }
            }

            let table_bytes = bincode::serialize(&table).unwrap();
            deps.storage.set(b"table", &table_bytes);

            return Ok(HandleResponse::default());
        }
        HandleMsg::Check {} => {
            let mut table: Table =
                bincode::deserialize(&deps.storage.get(b"table").unwrap()).unwrap();
            match table.stage {
                Stage::Ended {
                    is_draw: _,
                    winner: _,
                } => return Err(generic_err("The game is over.")),
                _ => { /* continue */ }
            };

            let me = deps.api.human_address(&env.message.sender).unwrap();

            if me != table.player_a && me != table.player_b {
                return Err(generic_err("You are not a player, go away!"));
            }

            if me != table.turn {
                return Err(generic_err("It's not your turn."));
            }

            if table.player_a_bet != table.player_b_bet {
                return Err(generic_err("You cannot check, must Call, Raise or Fold."));
            }

            if me == table.player_a {
                table.turn = table.player_b.clone();
            } else {
                table.turn = table.player_a.clone();
            }

            if table.turn == table.starter {
                // go to next stage
                table.stage = next(table.stage);
            }

            let table_bytes = bincode::serialize(&table).unwrap();
            deps.storage.set(b"table", &table_bytes);

            return Ok(HandleResponse::default());
        }
    }
}

fn next(s: Stage) -> Stage {
    match s {
        Stage::PreFlop => Stage::Flop,
        Stage::Flop => Stage::Turn,
        Stage::Turn => Stage::River,
        Stage::River => todo!(), // find winner
        Stage::Ended { is_draw, winner } => Stage::Ended {
            winner: winner.clone(),
            is_draw: is_draw,
        },
    }
}

/////////////////////////////// Query ///////////////////////////////
// These are getters, we only return what's public
// player get their private information as a response to txs (handle)
///////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetMyHand { secret: u64 },
    GetPublicData {},
}

pub fn query<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, msg: QueryMsg) -> QueryResult {
    match msg {
        QueryMsg::GetPublicData {} => {
            return Ok(QueryResponse::default());
        }
        QueryMsg::GetMyHand { secret } => {
            let secret_bytes = secret.to_be_bytes().to_vec();

            let player_a_secret = match deps.storage.get(b"player_a_secret") {
                None => {
                    return Err(generic_err(
                        "You are not a player, but there are still two seats left.",
                    ))
                }
                Some(x) => x,
            };
            let player_b_secret = match deps.storage.get(b"player_b_secret") {
                None => {
                    return Err(generic_err(
                        "You are not a player, but there is still one seat left.",
                    ))
                }
                Some(x) => x,
            };

            let first_card_index;
            let second_card_index;
            if secret_bytes == player_a_secret {
                first_card_index = PLAYER_A_FIRST_CARD;
                second_card_index = PLAYER_A_SECOND_CARD;
            } else if secret_bytes == player_b_secret {
                first_card_index = PLAYER_B_FIRST_CARD;
                second_card_index = PLAYER_B_SECOND_CARD;
            } else {
                return Err(generic_err("You are not a player, go away!"));
            }

            let deck_bytes = deps.storage.get(b"deck").unwrap();
            let deck: Vec<Card> = bincode::deserialize(&deck_bytes)
                .map_err(|e| generic_err(format!("Could not deserialze deck: {:?}", e)))?;

            let first_card: Card = deck[first_card_index];
            let second_card: Card = deck[second_card_index];

            return Ok(Binary(vec![
                first_card.value as u8,
                first_card.suit as u8,
                second_card.value as u8,
                second_card.suit as u8,
            ]));
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
