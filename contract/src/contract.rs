use bincode;
use cosmwasm_std::{
    generic_err, Api, Binary, CanonicalAddr, Env, Extern, HandleResponse, HandleResult, HumanAddr,
    InitResponse, InitResult, MigrateResponse, Querier, QueryResponse, QueryResult, StdResult,
    Storage,
};
use rand::{seq::SliceRandom, SeedableRng};
use rand_chacha::ChaChaRng;
use rs_poker::core::{Card, Deck, Rankable};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json;
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

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
enum Stage {
    PreFlop,
    Flop,
    Turn,
    River,
    EndedWinnerA,
    EndedWinnerB,
    EndedDraw,
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
                Stage::EndedWinnerA => return Err(generic_err("The game is over.")),
                Stage::EndedWinnerB => return Err(generic_err("The game is over.")),
                Stage::EndedDraw => return Err(generic_err("The game is over.")),
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
                Stage::EndedWinnerA => return Err(generic_err("The game is over.")),
                Stage::EndedWinnerB => return Err(generic_err("The game is over.")),
                Stage::EndedDraw => return Err(generic_err("The game is over.")),
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
                table.goto_next_stage(deps);
            }

            let table_bytes = bincode::serialize(&table).unwrap();
            deps.storage.set(b"table", &table_bytes);

            return Ok(HandleResponse::default());
        }
        HandleMsg::Fold {} => {
            let mut table: Table =
                bincode::deserialize(&deps.storage.get(b"table").unwrap()).unwrap();
            match table.stage {
                Stage::EndedWinnerA => return Err(generic_err("The game is over.")),
                Stage::EndedWinnerB => return Err(generic_err("The game is over.")),
                Stage::EndedDraw => return Err(generic_err("The game is over.")),
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
                table.stage = Stage::EndedWinnerB;
            } else {
                table.stage = Stage::EndedWinnerA;
            }

            let table_bytes = bincode::serialize(&table).unwrap();
            deps.storage.set(b"table", &table_bytes);

            return Ok(HandleResponse::default());
        }
        HandleMsg::Check {} => {
            let mut table: Table =
                bincode::deserialize(&deps.storage.get(b"table").unwrap()).unwrap();
            match table.stage {
                Stage::EndedWinnerA => return Err(generic_err("The game is over.")),
                Stage::EndedWinnerB => return Err(generic_err("The game is over.")),
                Stage::EndedDraw => return Err(generic_err("The game is over.")),
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
                table.goto_next_stage(deps);
            }

            let table_bytes = bincode::serialize(&table).unwrap();
            deps.storage.set(b"table", &table_bytes);

            return Ok(HandleResponse::default());
        }
    }
}

impl Table {
    fn goto_next_stage<S: Storage, A: Api, Q: Querier>(&mut self, deps: &mut Extern<S, A, Q>) {
        let deck_bytes = deps.storage.get(b"deck").unwrap();
        let deck: Vec<Card> = bincode::deserialize(&deck_bytes).unwrap();

        match self.stage {
            Stage::PreFlop => {
                self.stage = Stage::Flop;
                self.cards = vec![
                    deck[FLOP_FIRST_CARD],
                    deck[FLOP_SECOND_CARD],
                    deck[FLOP_THIRD_CARD],
                ];
            }
            Stage::Flop => {
                self.stage = Stage::Turn;
                self.cards = vec![
                    deck[FLOP_FIRST_CARD],
                    deck[FLOP_SECOND_CARD],
                    deck[FLOP_THIRD_CARD],
                    deck[TURN_CARD],
                ];
            }
            Stage::Turn => {
                self.stage = Stage::River;
                self.cards = vec![
                    deck[FLOP_FIRST_CARD],
                    deck[FLOP_SECOND_CARD],
                    deck[FLOP_THIRD_CARD],
                    deck[TURN_CARD],
                    deck[RIVER_CARD],
                ];
            }
            Stage::River => {
                let mut player_a_hand = self.cards.clone();
                player_a_hand.extend(vec![deck[PLAYER_A_FIRST_CARD], deck[PLAYER_A_SECOND_CARD]]);
                let player_a_rank = player_a_hand.rank();

                let mut player_b_hand = self.cards.clone();
                player_b_hand.extend(vec![deck[PLAYER_B_FIRST_CARD], deck[PLAYER_B_SECOND_CARD]]);
                let player_b_rank = player_b_hand.rank();

                if player_a_rank > player_b_rank {
                    self.stage = Stage::EndedWinnerA;
                } else if player_a_rank < player_b_rank {
                    self.stage = Stage::EndedWinnerB;
                } else {
                    self.stage = Stage::EndedDraw;
                }
                return;
            }
            _ => return,
        }

        if self.player_a_wallet == 0 || self.player_b_wallet == 0 {
            while self.stage != Stage::EndedDraw
                && self.stage != Stage::EndedWinnerA
                && self.stage != Stage::EndedWinnerB
            {
                self.goto_next_stage(deps);
            }
            // TODO find winner
            return;
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
    GetMyHand { secret: u64 },
    GetPublicData {},
}

pub fn query<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, msg: QueryMsg) -> QueryResult {
    match msg {
        QueryMsg::GetPublicData {} => {
            let table: Table = bincode::deserialize(&deps.storage.get(b"table").unwrap()).unwrap();
            return Ok(Binary(
                serde_json::to_string(&table)
                    .unwrap()
                    .as_str()
                    .as_bytes()
                    .to_vec(),
            ));
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
