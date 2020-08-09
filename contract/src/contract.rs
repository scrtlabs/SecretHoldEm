// use bincode;
use cosmwasm_std::{CosmosMsg, Coin, generic_err, Api, Binary, CanonicalAddr, Env, Extern, HandleResponse, HandleResult, HumanAddr, InitResponse, InitResult, MigrateResponse, Querier, QueryResult, StdResult, Storage, Uint128, BankMsg};
use rand::{seq::SliceRandom, SeedableRng};
use rand_chacha::ChaChaRng;
use rs_poker::core::{Card, Deck, Rankable};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json_wasm as serde_json;
use sha2::{Digest, Sha256};

#[derive(Serialize, Deserialize, Clone)]
struct Table {
    game_counter: u64,

    player_a: Option<HumanAddr>,
    player_a_wallet: i128,
    player_a_bet: i64,

    player_b: Option<HumanAddr>,
    player_b_wallet: i128,
    player_b_bet: i64,

    starter: Option<HumanAddr>,
    turn: Option<HumanAddr>, // round ends if after a bet: starter != turn && player_a_bet == player_b_bet or if someone called
    last_play: Option<String>,

    stage: Stage,

    community_cards: Vec<Card>,

    player_a_hand: Vec<Card>,
    player_b_hand: Vec<Card>,

    player_a_wants_rematch: bool,
    player_b_wants_rematch: bool,

    player_a_win_counter: u64,
    player_b_win_counter: u64,
    tie_counter: u64,
    max_credit: u64,
    min_credit: u64,
    big_blind: u64,
}

// struct Player {
//     address: Option<HumanAddr>,
//     position: u8,
//     wallet: u128,
//     current_bet: i64
// }

/////////////////////////////// Init ///////////////////////////////
//
////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InitMsg {
    big_blind: u64
}

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> InitResult {
    let table = Table {
        game_counter: 0,

        player_a: None,
        player_b: None,

        player_a_wallet: 0,
        player_b_wallet: 0,

        player_a_bet: 0,
        player_b_bet: 0,

        stage: Stage::WaitingForPlayersToJoin,
        starter: None,
        turn: None,
        last_play: None,

        community_cards: vec![],

        player_a_hand: vec![],
        player_b_hand: vec![],

        player_a_wants_rematch: false,
        player_b_wants_rematch: false,

        player_a_win_counter: 0,
        player_b_win_counter: 0,
        tie_counter: 0,
        max_credit: msg.big_blind * (MAX_TABLE_BIG_BLINDS as u64),
        min_credit: msg.big_blind * (MIN_TABLE_BIG_BLINDS as u64),
        big_blind: msg.big_blind
    };

    deps.storage
        .set(b"table", &serde_json::to_vec(&table).unwrap());

    Ok(InitResponse::default())
}

/////////////////////////////// Handle ///////////////////////////////
//
//////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
#[repr(u8)]
enum Stage {
    WaitingForPlayersToJoin,
    PreFlop,
    Flop,
    Turn,
    River,
    EndedWinnerA,
    EndedWinnerB,
    EndedDraw,
}

impl Stage {
    fn no_more_action(&self) -> bool {
        match &self {
            Self::EndedWinnerA | Self::EndedWinnerB | Self::EndedDraw | Self::WaitingForPlayersToJoin => true,
            _ => false,
        }
    }

    fn next_round(&self) -> Self {
        match &self {
            Self::PreFlop => Self::Flop,
            Self::Flop => Self::Turn,
            Self::Turn => Self::River,
            _ => Self::PreFlop
        }
    }
}

const MAX_TABLE_BIG_BLINDS: u8 = 100;
const MIN_TABLE_BIG_BLINDS: u8 = 20;
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

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Join { secret: u64 },
    Raise { amount: u64 },
    Call {},
    Fold {},
    Check {},
    Rematch {},
    Withdraw {},
    TopUp {},
}

pub fn winner_winner_chicken_dinner(contract_address: HumanAddr, player: HumanAddr, amount: Uint128) -> HandleResponse {
    HandleResponse{
        messages: vec![CosmosMsg::Bank(BankMsg::Send {
            from_address: contract_address,
            to_address: player,
            amount: vec![Coin{
                denom: "uscrt".to_string(),
                amount,
            }]}),
        ],
        log: vec![],
        data: None
    }
}

fn can_deposit(env: &Env, table: &Table, current_amount: i128) -> StdResult<i128> {

    let deposit: Uint128;

    if env.message.sent_funds.len() == 0 {
        return Err(generic_err("SHOW ME THE MONEY"));
    } else {
        if env.message.sent_funds[0].denom != "uscrt" {
            return Err(generic_err("WRONG MONEY"));
        }
        deposit = env.message.sent_funds[0].amount;

        if deposit.u128() as i128 + current_amount < table.min_credit as i128 {
            return Err(generic_err("GTFO DIRTY SHORT STACKER"));
        }

        if deposit.u128() as i128 + current_amount > table.max_credit as i128 {
            return Err(generic_err("GTFO DIRTY DEEP STACKER"));
        }
    }
    Ok(deposit.u128() as i128)
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> HandleResult {
    return match msg {
        HandleMsg::TopUp {} => {

            let player_name = deps.api.human_address(&env.message.sender)?;

            let mut table: Table =
                serde_json::from_slice(&deps.storage.get(b"table").unwrap()).unwrap();

            let pb = (&table).player_b.clone().unwrap_or(HumanAddr::default());
            let pa = (&table).player_a.clone().unwrap_or(HumanAddr::default());

            if player_name == pb {
                let deposit = can_deposit(&env, &table, table.player_b_wallet)?;
                table.player_b_wallet += deposit;
            } else if player_name == pa {
                let deposit = can_deposit(&env, &table, table.player_a_wallet)?;
                table.player_a_wallet += deposit;
            } else {
                return Err(generic_err("You are not a player, or you are broke! Either way, go away!"));
            }

            Ok(HandleResponse::default())
        }
        HandleMsg::Withdraw {} => {
            let player_name = deps.api.human_address(&env.message.sender)?;
            let contract_address = deps.api.human_address(&env.contract.address)?;

            let mut table: Table =
                serde_json::from_slice(&deps.storage.get(b"table").unwrap()).unwrap();

            if player_name == table.player_b.unwrap() && table.player_b_wallet != 0 {

                //fold player b
                if !table.stage.no_more_action() {
                    table.stage = Stage::EndedWinnerA;
                    table.player_a_wallet += (table.player_a_bet + table.player_b_bet) as i128;
                    table.player_a_win_counter += 1;
                    table.last_play = Some(String::from("Player B folded"));
                }
                let amount = table.player_b_wallet;

                return Ok(winner_winner_chicken_dinner(contract_address, player_name, Uint128(amount as u128)));
            } else if player_name == table.player_a.unwrap() && table.player_a_wallet != 0 {

                //fold player a
                if !table.stage.no_more_action() {
                    table.stage = Stage::EndedWinnerB;
                    table.player_b_wallet += (table.player_a_bet + table.player_b_bet) as i128;
                    table.player_b_win_counter += 1;
                    table.last_play = Some(String::from("Player B folded"));
                }
                let amount = table.player_a_wallet;

                return Ok(winner_winner_chicken_dinner(contract_address, player_name, Uint128(amount as u128)));
            }

            Err(generic_err("You are not a player, or you are broke! Either way, go away!"))
        },
        HandleMsg::Join { secret } => {

            let mut table: Table =
                serde_json::from_slice(&deps.storage.get(b"table").unwrap()).unwrap();

            let deposit = can_deposit(&env, &table, 0)?;

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

                let a_human_addr = deps
                    .api
                    .human_address(&CanonicalAddr(Binary(player_name.to_vec())))
                    .unwrap();


                table.player_a = Some(a_human_addr.clone());
                table.player_a_wallet = deposit;
                table.starter = Some(a_human_addr.clone());
                table.turn = Some(a_human_addr.clone());
                deps.storage
                    .set(b"table", &serde_json::to_vec(&table).unwrap());

                return Ok(HandleResponse::default());
            }

            // player b - we can now shuffle the deck

            deps.storage.set(b"player_b", player_name);
            deps.storage.set(b"player_b_secret", player_secret);

            let player_a_secret = deps.storage.get(b"player_a_secret").unwrap();

            let mut combined_secret = player_a_secret.clone();
            combined_secret.extend(player_secret);
            combined_secret.extend(&(0 as u64).to_be_bytes()); // game counter
            let seed: [u8; 32] = Sha256::digest(&combined_secret).into();

            let mut rng = ChaChaRng::from_seed(seed);
            let mut deck: Vec<Card> = Deck::default().into_iter().collect();
            deck.shuffle(&mut rng);

            deps.storage
                .set(b"deck", &serde_json::to_vec(&deck).unwrap());

            let a_human_addr = deps
                .api
                .human_address(&CanonicalAddr(Binary(player_a.expect("Error"))))
                .unwrap();
            let b_human_addr = deps
                .api
                .human_address(&CanonicalAddr(Binary(player_name.to_vec())))
                .unwrap();

            table.player_b = Some(b_human_addr);
            table.player_b_wallet = deposit;

            table.stage = table.stage.next_round();
            table.starter = Some(a_human_addr.clone());
            table.turn = Some(a_human_addr.clone());

            deps.storage
                .set(b"table", &serde_json::to_vec(&table).unwrap());

            Ok(HandleResponse::default())
        }
        HandleMsg::Raise { amount } => {
            let mut table: Table =
                serde_json::from_slice(&deps.storage.get(b"table").unwrap()).unwrap();
            if table.stage.no_more_action() {
                return Err(generic_err("Action hasn't started yet"));
            }

            let me = Some(deps.api.human_address(&env.message.sender).unwrap());

            if me != table.player_a && me != table.player_b {
                return Err(generic_err("You are not a player, go away!"));
            }

            if me != table.turn {
                return Err(generic_err("It's not your turn."));
            }

            if me == table.player_a {

                if table.player_a_wallet < amount as i128 {
                    return Err(generic_err("You cannot raise more than you have!"));
                }

                // I'm player A
                table.player_a_wallet -= (table.player_b_bet + amount as i64 - table.player_a_bet) as i128;
                if table.player_a_wallet < 0 {
                    return Err(generic_err(
                        "You don't have enough credits to raise by that much.",
                    ));
                }
                table.player_a_bet = table.player_b_bet + amount as i64;

                table.last_play = Some(String::from(format!(
                    "Player A raised by {} credits",
                    amount
                )));
                table.turn = table.player_b.clone();
            } else {

                // I'm player B
                if table.player_b_wallet < amount as i128 {
                    return Err(generic_err("You cannot raise more than you have!"));
                }

                table.player_b_wallet -= (table.player_a_bet + amount as i64 - table.player_b_bet) as i128;
                if table.player_b_wallet < 0 {
                    return Err(generic_err(
                        "You don't have enough credits to raise by that much.",
                    ));
                }
                table.player_b_bet = table.player_a_bet + amount as i64;

                table.last_play = Some(String::from(format!(
                    "Player B raised by {} credits",
                    amount
                )));
                table.turn = table.player_a.clone();
            }

            deps.storage
                .set(b"table", &serde_json::to_vec(&table).unwrap());

            Ok(HandleResponse::default())
        }
        HandleMsg::Call {} => {
            let mut table: Table =
                serde_json::from_slice(&deps.storage.get(b"table").unwrap()).unwrap();
            if table.stage.no_more_action() {
                return Err(generic_err("Action hasn't started yet"));
            }

            let me = Some(deps.api.human_address(&env.message.sender).unwrap());

            if me != table.player_a && me != table.player_b {
                return Err(generic_err("You are not a player, go away!"));
            }

            if me != table.turn {
                return Err(generic_err("It's not your turn."));
            }

            if me == table.player_a {
                // I'm player A
                table.player_a_wallet -= (table.player_b_bet - table.player_a_bet) as i128;
                if table.player_a_wallet < 0 {
                    return Err(generic_err(
                        "You cannot Call, your bet is bigger or equals to the other player's bet.",
                    ));
                }
                table.player_a_bet = table.player_b_bet;

                table.last_play = Some(String::from("Player A called"));
            } else {
                // I'm player B
                table.player_b_wallet -= (table.player_a_bet - table.player_b_bet) as i128;
                if table.player_b_wallet < 0 {
                    return Err(generic_err(
                        "You cannot Call, your bet is bigger or equals to the other player's bet.",
                    ));
                }
                table.player_b_bet = table.player_a_bet;

                table.last_play = Some(String::from("Player B called"));
            }

            table.turn = table.player_a.clone();
            table.goto_next_stage(deps);

            deps.storage
                .set(b"table", &serde_json::to_vec(&table).unwrap());

            Ok(HandleResponse::default())
        }
        HandleMsg::Fold {} => {
            let mut table: Table =
                serde_json::from_slice(&deps.storage.get(b"table").unwrap()).unwrap();
            if table.stage.no_more_action() {
                return Err(generic_err("Action hasn't started yet"));
            }

            let me = Some(deps.api.human_address(&env.message.sender).unwrap());

            if me != table.player_a && me != table.player_b {
                return Err(generic_err("You are not a player, go away!"));
            }

            if me != table.turn {
                return Err(generic_err("It's not your turn."));
            }

            if me == table.player_a {
                table.stage = Stage::EndedWinnerB;
                table.player_b_wallet += (table.player_a_bet + table.player_b_bet) as i128;
                table.player_b_win_counter += 1;
                table.last_play = Some(String::from("Player A folded"));
            } else {
                table.stage = Stage::EndedWinnerA;
                table.player_a_wallet += (table.player_a_bet + table.player_b_bet) as i128;
                table.player_a_win_counter += 1;
                table.last_play = Some(String::from("Player B folded"));
            }

            deps.storage
                .set(b"table", &serde_json::to_vec(&table).unwrap());

            Ok(HandleResponse::default())
        }
        HandleMsg::Check {} => {
            let mut table: Table =
                serde_json::from_slice(&deps.storage.get(b"table").unwrap()).unwrap();
            if table.stage.no_more_action() {
                return Err(generic_err("Action hasn't started yet"));
            }

            let me = Some(deps.api.human_address(&env.message.sender).unwrap());

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
                table.last_play = Some(String::from("Player A checked"));
                table.turn = table.player_b.clone();
            } else {
                table.last_play = Some(String::from("Player B checked"));
                table.turn = table.player_a.clone();
            }

            if table.turn == table.starter {
                table.goto_next_stage(deps);
            }

            deps.storage
                .set(b"table", &serde_json::to_vec(&table).unwrap());

            Ok(HandleResponse::default())
        }
        HandleMsg::Rematch {} => {
            let mut table: Table =
                serde_json::from_slice(&deps.storage.get(b"table").unwrap()).unwrap();

            if !table.stage.no_more_action() {
                return Err(generic_err("You can't start a new game now!"))
            }

            let me = Some(deps.api.human_address(&env.message.sender).unwrap());

            if me != table.player_a && me != table.player_b {
                return Err(generic_err("You are not a player, go away!"));
            }

            if table.player_b_wallet == 0 || table.player_a_wallet == 0 {
                return Err(generic_err("One of the players must deposit to continue playing"));
            }

            if me == table.player_a {
                table.player_a_wants_rematch = true;
            } else {
                table.player_b_wants_rematch = true;
            }

            if !table.player_b_wants_rematch || !table.player_a_wants_rematch {
                // not everyone approved a rematch yet
                deps.storage
                    .set(b"table", &serde_json::to_vec(&table).unwrap());
                return Ok(HandleResponse::default());
            }

            table.game_counter += 1;

            let player_a_secret = deps.storage.get(b"player_a_secret").unwrap();
            let player_b_secret = deps.storage.get(b"player_b_secret").unwrap();

            let mut combined_secret = player_a_secret.clone();
            combined_secret.extend(player_b_secret);
            combined_secret.extend(&table.game_counter.to_be_bytes()); // game counter
            let seed: [u8; 32] = Sha256::digest(&combined_secret).into();

            let mut rng = ChaChaRng::from_seed(seed);
            let mut deck: Vec<Card> = Deck::default().into_iter().collect();
            deck.shuffle(&mut rng);

            deps.storage
                .set(b"deck", &serde_json::to_vec(&deck).unwrap());

            table.stage = Stage::PreFlop;
            table.turn = table.starter.clone();
            table.last_play = None;

            table.community_cards = vec![];

            table.player_a_bet = 0;
            table.player_b_bet = 0;

            table.player_a_hand = vec![];
            table.player_b_hand = vec![];

            table.player_a_wants_rematch = false;
            table.player_b_wants_rematch = false;

            deps.storage
                .set(b"table", &serde_json::to_vec(&table).unwrap());

            Ok(HandleResponse::default())
        }
    }
}

impl Table {
    fn goto_next_stage<S: Storage, A: Api, Q: Querier>(&mut self, deps: &mut Extern<S, A, Q>) {
        let deck: Vec<Card> = serde_json::from_slice(&deps.storage.get(b"deck").unwrap()).unwrap();

        match self.stage {
            Stage::PreFlop => {
                self.stage = Stage::Flop;
                self.community_cards = vec![
                    deck[FLOP_FIRST_CARD],
                    deck[FLOP_SECOND_CARD],
                    deck[FLOP_THIRD_CARD],
                ];
            }
            Stage::Flop => {
                self.stage = Stage::Turn;
                self.community_cards = vec![
                    deck[FLOP_FIRST_CARD],
                    deck[FLOP_SECOND_CARD],
                    deck[FLOP_THIRD_CARD],
                    deck[TURN_CARD],
                ];
            }
            Stage::Turn => {
                self.stage = Stage::River;
                self.community_cards = vec![
                    deck[FLOP_FIRST_CARD],
                    deck[FLOP_SECOND_CARD],
                    deck[FLOP_THIRD_CARD],
                    deck[TURN_CARD],
                    deck[RIVER_CARD],
                ];
            }
            Stage::River => {
                let mut player_a_7_card_hand = self.community_cards.clone();
                player_a_7_card_hand
                    .extend(vec![deck[PLAYER_A_FIRST_CARD], deck[PLAYER_A_SECOND_CARD]]);
                let player_a_rank = player_a_7_card_hand.rank();

                let mut player_b_7_card_hand = self.community_cards.clone();
                player_b_7_card_hand
                    .extend(vec![deck[PLAYER_B_FIRST_CARD], deck[PLAYER_B_SECOND_CARD]]);
                let player_b_rank = player_b_7_card_hand.rank();

                if player_a_rank > player_b_rank {
                    self.stage = Stage::EndedWinnerA;
                    self.player_a_wallet += (self.player_a_bet + self.player_b_bet) as i128;
                    self.player_a_win_counter += 1;
                } else if player_a_rank < player_b_rank {
                    self.stage = Stage::EndedWinnerB;
                    self.player_b_wallet += (self.player_a_bet + self.player_b_bet) as i128;
                    self.player_b_win_counter += 1;
                } else {
                    self.stage = Stage::EndedDraw;
                    self.player_a_wallet += self.player_a_bet as i128;
                    self.player_b_wallet += self.player_b_bet as i128;
                    self.tie_counter += 1;
                }

                self.player_a_hand = vec![deck[PLAYER_A_FIRST_CARD], deck[PLAYER_A_SECOND_CARD]];
                self.player_b_hand = vec![deck[PLAYER_B_FIRST_CARD], deck[PLAYER_B_SECOND_CARD]];
                return;
            }
            Stage::WaitingForPlayersToJoin => {
                return;
            }
            Stage::EndedWinnerA => {
                return;
            }
            Stage::EndedWinnerB => {
                return;
            }
            Stage::EndedDraw => {
                return;
            }
        }

        // Turn ended with both player out of cash, just play it out
        if self.player_a_wallet == 0 || self.player_b_wallet == 0 {
            while self.stage != Stage::EndedDraw
                && self.stage != Stage::EndedWinnerA
                && self.stage != Stage::EndedWinnerB
            {
                self.goto_next_stage(deps);
            }
            return;
        }
    }
}

/////////////////////////////// Query ///////////////////////////////
// These are getters, we only return what's public
// player get their private information as a response to txs (handle)
///////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetMyHand { secret: u64 },
    GetPublicData {},
}

pub fn query<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, msg: QueryMsg) -> QueryResult {
    match msg {
        QueryMsg::GetPublicData {} => {
            return Ok(Binary(deps.storage.get(b"table").unwrap()));
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

            let deck: Vec<Card> =
                serde_json::from_slice(&deps.storage.get(b"deck").unwrap()).unwrap();

            let first_card: Card = deck[first_card_index];
            let second_card: Card = deck[second_card_index];

            return Ok(Binary(
                serde_json::to_vec(&vec![first_card, second_card]).unwrap(),
            ));
        }
    }
}

/////////////////////////////// Migrate ///////////////////////////////
// Isn't supported by the Secret Network, but we must declare this to
// comply with CosmWasm 0.9 API
///////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MigrateMsg {}

pub fn migrate<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    _msg: MigrateMsg,
) -> StdResult<MigrateResponse> {
    Ok(MigrateResponse::default())
}
