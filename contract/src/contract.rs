// use bincode;
use cosmwasm_std::{CosmosMsg, Coin, generic_err, Api, Binary, CanonicalAddr, Env, Extern, HandleResponse, HandleResult, HumanAddr, InitResponse, InitResult, MigrateResponse, Querier, QueryResult, StdResult, Storage, Uint128, BankMsg};
use rand::{seq::SliceRandom, SeedableRng};
use rand_chacha::ChaChaRng;
use rs_poker::core::{Card, Deck, Rankable};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json_wasm as serde_json;
use sha2::{Digest, Sha256};
use core::cmp::max;

#[derive(Serialize, Deserialize, Clone)]
struct Table {
    game_counter: u64,

    starter: Option<HumanAddr>,
    turn: Option<HumanAddr>, // round ends if after a bet: starter != turn && player_a_bet == player_b_bet or if someone called
    last_play: Option<String>,

    stage: Stage,

    community_cards: Vec<Card>,

    button: u8,
    to_act: u8,

    pot: i128,
    bet: u64,

    winners: Vec<u8>,

    players: [Option<Player>; 9],

    secrets: Vec<u8>,

    max_raise_amt: i128,

    max_credit: u64,
    min_credit: u64,
    big_blind: u64,
}

impl Table {

    fn betting_round_over(&self) -> bool {
        return self.to_act == 255
    }

    // fn check_all_in(&self) -> (bool, u8) {
    //     let mut count = 0;
    //     let mut last_seen: u8 = 0;
    //     for player in self.players.iter() {
    //         if let Some(p) = player {
    //             if p.in_play {
    //                 count += 1;
    //                 last_seen = p.seat;
    //             }
    //         }
    //     }
    //     (count == 1, last_seen)
    // }

    fn check_all_folded(&self) -> (bool, u8) {
        let mut count = 0;
        let mut last_seen: u8 = 0;
        for player in self.players.iter() {
            if let Some(p) = player {
                if p.in_play {
                    count += 1;
                    last_seen = p.seat;
                }
            }
        }
        (count == 1, last_seen)
    }

    fn next_active_player(&self, starting_pos: u8) -> u8 {
        let mut pos: u8 = starting_pos;

        loop {
            pos = self.next_player(pos);
            if pos == starting_pos {
                // no active players found
                return 255
            }

            let player: &Player = self.players.get(pos).unwrap();
            if player.in_play && player.current_bet != self.bet {
                return pos
            }
        }
    }

    fn fold(&mut self) {
        let player: &mut Player = self.players.get_mut(self.to_act).unwrap();

        player.in_play = false;

        self.to_act = self.next_active_player(self.to_act);
    }

    fn call(&mut self) {
        let player: &mut Player = self.players.get_mut(self.to_act).unwrap();

        player.bet(self.bet as i64);

        self.to_act = self.next_active_player(self.to_act);
    }

    fn raise(&mut self, amount: u64) {
        let player: &mut Player = self.players.get_mut(self.to_act).unwrap();

        player.bet(amount as i64);
        self.bet += amount;

        self.to_act = self.next_active_player(self.to_act);
    }

    fn is_it_my_turn(&self, me: &HumanAddr) -> bool {
        let player: &Player = self.players.get(self.to_act).unwrap();
        return &player.address == me;
    }

    fn get_player_by_secret(&self, secret: u64) -> StdResult<Player> {
        for player in self.players.iter() {
            if let Some(p) = player {
                if p.secret == secret.to_be_bytes().to_vec() {
                    return Ok(p.clone());
                }
            }
        }
        return Err(generic_err("You aren't in this game bro"))
    }
    fn filter_private_data(&mut self) {
        match self.stage {
            Stage::PreFlop => {
                self.community_cards = vec![];
            },
            Stage::Flop => {
                self.community_cards.pop();
                self.community_cards.pop();
            }
            Stage::Turn => {
                self.community_cards.pop();
            }
            _ => {},
        }

        for player in self.players.iter_mut() {
            if let Some(p) = player {
                p.hand = vec![];
                p.secret = vec![];
            }
        }

    }

    fn deal_hands(&mut self) -> () {
        let seed: [u8; 32] = Sha256::digest(&self.secrets).into();

        let deck = MyDeck::new_shuffled(seed);

        let mut iter = deck.0.iter();

        for player in self.players.iter_mut() {
            if let Some(p) = player {
                p.hand.push(iter.next().unwrap().clone());
                p.hand.push(iter.next().unwrap().clone());
            }
        }

        self.community_cards.push(iter.next().unwrap().clone());
        self.community_cards.push(iter.next().unwrap().clone());
        self.community_cards.push(iter.next().unwrap().clone());
        self.community_cards.push(iter.next().unwrap().clone());
        self.community_cards.push(iter.next().unwrap().clone());

    }

    fn start_round(&mut self) {
        self.next_round();
    }

    fn sit_down(&mut self, seat: u8, p: Player) {
        self.players[seat] = Some(p);
    }

    fn num_of_players(&self) -> u8 {
        let mut count = 0;
        for player in self.players.iter() {
            if let Some(p) = player {
                count += 1;
            }
        }

        count
    }

    fn is_seat_taken(&self, seat: u8) -> bool {
        return self.players.get(seat).is_some();
    }

    fn goto_next_stage<S: Storage, A: Api, Q: Querier>(&mut self, deps: &mut Extern<S, A, Q>) {

        let (all_folded, winner) = self.check_all_folded();
        if all_folded {
            self.winners.push(winner);
            self.distribute_pot();
            Stage::EndedDraw;
            return;
        }

        match self.stage {
            Stage::PreFlop => {
                self.stage = Stage::Flop;
            }
            Stage::Flop => {
                self.stage = Stage::Turn;
            }
            Stage::Turn => {
                self.stage = Stage::River;
            }
            Stage::River => {
                self.showdown()
            }
            _ => {
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

    fn next_player(&self, starting_pos: u8) -> u8 {

        let mut iter = self.players.iter().cycle();
        iter.skip(starting_pos as usize);

        for pos in 0..9 {
            if let Some(Some(_)) = iter.next() {
                return (starting_pos + pos) % 9;
            }
        }
        return 0;
    }

    fn next_round(&mut self) {
        self.game_counter += 1;
        self.button += self.next_player(self.button);
        self.stage = Stage::PreFlop;
        self.pot = 0;
        self.bet = self.big_blind;

        let (sb, bb) = self.blinds();

        for player in self.players.iter_mut() {
            if let Some(p) = player {
                p.current_bet = 0;
                p.in_play = true;

                if p.seat == sb {
                    p.bet((self.big_blind / 2) as i64);
                } else if p.seat == bb {
                    p.bet(self.big_blind as i64);
                }
                self.max_raise_amt = max(self.max_raise_amt, p.wallet)
            }

        }

        self.to_act = self.next_player(bb);
    }

    fn blinds(&self) -> (u8, u8) {
        return if self.players.len() == 2 {
            (self.button, self.next_player(self.button))
        } else {
            (self.next_player(self.button), self.next_player(self.next_player(self.button)))
        }
    }

    fn showdown(&mut self) {

        let mut winning_hand = self.community_cards.clone().rank();
        self.winners = vec![];
        for player in self.players.iter_mut() {

            let mut player_hand = self.community_cards.clone();
            if let Some(p) = player {
                if p.in_play {
                    player_hand.extend(&p.hand);

                    if winning_hand < player_hand.rank() {
                        winning_hand = player_hand.rank();
                        winners.push(p.seat);
                    } else if winning_hand = player_hand.rank() {
                        winners.push(p.seat);
                    }
                }
            }
        }

        self.stage = Stage::EndedDraw

    }

    fn distribute_pot(&mut self) {
        let num_of_winners = self.winners.len();
        if num_of_winners == 0 {
            return;
        }

        for seat in self.winners.iter() {
            if let Some(Some(mut x)) = self.players.get(seat) {
                x.wallet += self.pot / num_of_winners;
            }
        }
    }
}

impl Default for Table {
    fn default() -> Self {
        return Self {
            game_counter: 0,

            stage: Stage::WaitingForPlayersToJoin,
            starter: None,
            turn: None,
            last_play: None,

            community_cards: vec![],

            button: 0,
            to_act: 0,
            pot: 0,
            bet: 0,
            winners: vec![],
            players: [None, None, None, None, None, None, None, None, None],
            secrets: vec![],
            max_raise_amt: 0,
            max_credit: 0,
            min_credit: 0,
            big_blind: 0
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct Player {
    address: HumanAddr,
    seat: u8,
    wallet: i128,
    current_bet: i64,
    in_play: bool,
    hand: Vec<Card>,
    secret: Vec<u8>
}

impl Player {
    fn bet(&mut self, amount: i64) -> i64 {
        self.current_bet += amount;
        self.wallet -= amount;

        return amount;
    }
}

impl Default for Player {
    fn default() -> Self {
        Self {
            address: HumanAddr::default(),
            seat: 0,
            wallet: 0,
            in_play: false,
            current_bet: 0,
            hand: vec![],
            secret: vec![]
        }
    }
}

struct MyDeck(pub Vec<Card>);

impl MyDeck {
    fn new_shuffled(seed: [u8; 32]) -> Self {
        let mut rng = ChaChaRng::from_seed(seed);
        let mut deck: Vec<Card> = Deck::default().into_iter().collect();
        deck.shuffle(&mut rng);

        Self(deck)
    }
}


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
    let mut table = Table::default();

    table.max_credit = msg.big_blind * (MAX_TABLE_BIG_BLINDS as u64);
    table.min_credit = msg.big_blind * (MIN_TABLE_BIG_BLINDS as u64);
    table.big_blind = msg.big_blind;

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
    Join { secret: u64, seat: u8 },
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
        HandleMsg::Join { secret, seat } => {

            let mut table: Table =
                serde_json::from_slice(&deps.storage.get(b"table").unwrap()).unwrap();

            if table.is_seat_taken(seat) {
                return Err(generic_err("Seat is taken :("));
            }


            let deposit = can_deposit(&env, &table, 0)?;

            let player_a = deps.storage.get(b"player_a");
            let player_b = deps.storage.get(b"player_b");

            if player_a.is_some() && player_b.is_some() {
                return Err(generic_err("Table is full."));
            }

            let player_name = deps.api.human_address(&env.message.sender)?;

            let new_player = Player {
                address: player_name,
                seat,
                wallet: deposit,
                current_bet: 0,
                in_play: false,
                hand: vec![],
                secret: secret.to_be_bytes().to_vec()
            };

            table.sit_down(seat, new_player);

            Ok(HandleResponse::default())
        }

        HandleMsg::Raise { amount } => {
            let mut table: Table =
                serde_json::from_slice(&deps.storage.get(b"table").unwrap()).unwrap();
            if table.stage.no_more_action() {
                return Err(generic_err("Action hasn't started yet"));
            }

            let me = deps.api.human_address(&env.message.sender)?;

            if !table.is_it_my_turn(&me) {
                return Err(generic_err("Action isn't on you"));
            }

            table.raise(amount);

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

            let me = deps.api.human_address(&env.message.sender)?;

            if !table.is_it_my_turn(&me) {
                return Err(generic_err("Action isn't on you"));
            }

            table.call();

            if table.betting_round_over() {
                table.goto_next_stage(deps);
            }

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

            let me = deps.api.human_address(&env.message.sender)?;

            if !table.is_it_my_turn(&me) {
                return Err(generic_err("Action isn't on you"));
            }

            table.fold();

            if table.betting_round_over() {
                table.goto_next_stage(deps);
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
    return match msg {
        QueryMsg::GetPublicData {} => {
            let mut table: Table =
                serde_json::from_slice(&deps.storage.get(b"table").unwrap()).unwrap();

            table.filter_private_data();

            Ok(Binary(serde_json::to_vec(&table).unwrap()))
        }
        QueryMsg::GetMyHand { secret } => {
            let secret_bytes = secret.to_be_bytes().to_vec();

            let mut table: Table =
                serde_json::from_slice(&deps.storage.get(b"table").unwrap()).unwrap();

            let player = table.get_player_by_secret(secret)?;

            Ok(Binary(
                serde_json::to_vec(&player.hand).unwrap(),
            ))
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
