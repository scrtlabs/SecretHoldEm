use bincode;
use cosmwasm_std::{
    generic_err, Api, Binary, Env, Extern, HandleResponse, HandleResult, InitResponse, InitResult,
    MigrateResponse, Querier, QueryResponse, QueryResult, StdResult, Storage,
};
use rand::{seq::SliceRandom, SeedableRng};
use rand_chacha::ChaChaRng;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/////////////////////////////// Init ///////////////////////////////
// creates a game and joins as the first player
////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InitMsg {
    CreateTable {},
}

pub fn init<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> InitResult {
    match msg {
        InitMsg::CreateTable {} => Ok(InitResponse::default()),
    }
}

/////////////////////////////// Handle ///////////////////////////////
//
//////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Join { secret: u64 },
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
                // player a
                deps.storage.set(b"player_a", player_name);
                deps.storage.set(b"player_a_secret", player_secret);
                return Ok(HandleResponse::default());
            }

            // player b - can now shuffle the deck

            deps.storage.set(b"player_b", player_name);
            deps.storage.set(b"player_b_secret", player_secret);

            let player_a_secret = deps.storage.get(b"player_a_secret").unwrap();

            let mut combined_secret = player_a_secret.clone();
            combined_secret.extend(player_secret);
            let shuffle_seed: [u8; 32] = Sha256::digest(&combined_secret).into();

            let mut rng = ChaChaRng::from_seed(shuffle_seed);
            let mut deck = get_new_deck();
            deck.shuffle(&mut rng);

            let deck_bytes = bincode::serialize(&deck).unwrap();
            deps.storage.set(b"deck", &deck_bytes);

            return Ok(HandleResponse::default());
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
    GetAllPublicData {},
}

pub fn query<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, msg: QueryMsg) -> QueryResult {
    match msg {
        QueryMsg::GetAllPublicData {} => {
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
                first_card_index = 0;
                second_card_index = 2;
            } else if secret_bytes == player_b_secret {
                first_card_index = 1;
                second_card_index = 3;
            } else {
                return Err(generic_err("You are not a player, go away!"));
            }

            let deck_bytes = deps.storage.get(b"deck").unwrap();
            let deck: Vec<Card> = bincode::deserialize(&deck_bytes)
                .map_err(|e| generic_err(format!("Could not deserialze deck: {:?}", e)))?;

            let first_card = (&deck)[first_card_index];
            let second_card = (&deck)[second_card_index];

            return Ok(Binary(vec![
                first_card.number as u8,
                first_card.shape as u8,
                second_card.number as u8,
                second_card.shape as u8,
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Copy)]
struct Card {
    pub number: Number,
    pub shape: Shape,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Copy)]
#[repr(u8)]
enum Number {
    Ace = 1,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Copy)]
#[repr(u8)]
enum Shape {
    Spades = 1,
    Hearts,
    Clubs,
    Diamonds,
}
fn get_new_deck() -> Vec<Card> {
    return vec![
        Card {
            number: Number::Ace,
            shape: Shape::Clubs,
        },
        Card {
            number: Number::Two,
            shape: Shape::Clubs,
        },
        Card {
            number: Number::Three,
            shape: Shape::Clubs,
        },
        Card {
            number: Number::Four,
            shape: Shape::Clubs,
        },
        Card {
            number: Number::Five,
            shape: Shape::Clubs,
        },
        Card {
            number: Number::Six,
            shape: Shape::Clubs,
        },
        Card {
            number: Number::Seven,
            shape: Shape::Clubs,
        },
        Card {
            number: Number::Eight,
            shape: Shape::Clubs,
        },
        Card {
            number: Number::Nine,
            shape: Shape::Clubs,
        },
        Card {
            number: Number::Ten,
            shape: Shape::Clubs,
        },
        Card {
            number: Number::Jack,
            shape: Shape::Clubs,
        },
        Card {
            number: Number::Queen,
            shape: Shape::Clubs,
        },
        Card {
            number: Number::King,
            shape: Shape::Clubs,
        },
        Card {
            number: Number::Ace,
            shape: Shape::Hearts,
        },
        Card {
            number: Number::Two,
            shape: Shape::Hearts,
        },
        Card {
            number: Number::Three,
            shape: Shape::Hearts,
        },
        Card {
            number: Number::Four,
            shape: Shape::Hearts,
        },
        Card {
            number: Number::Five,
            shape: Shape::Hearts,
        },
        Card {
            number: Number::Six,
            shape: Shape::Hearts,
        },
        Card {
            number: Number::Seven,
            shape: Shape::Hearts,
        },
        Card {
            number: Number::Eight,
            shape: Shape::Hearts,
        },
        Card {
            number: Number::Nine,
            shape: Shape::Hearts,
        },
        Card {
            number: Number::Ten,
            shape: Shape::Hearts,
        },
        Card {
            number: Number::Jack,
            shape: Shape::Hearts,
        },
        Card {
            number: Number::Queen,
            shape: Shape::Hearts,
        },
        Card {
            number: Number::King,
            shape: Shape::Hearts,
        },
        Card {
            number: Number::Ace,
            shape: Shape::Spades,
        },
        Card {
            number: Number::Two,
            shape: Shape::Spades,
        },
        Card {
            number: Number::Three,
            shape: Shape::Spades,
        },
        Card {
            number: Number::Four,
            shape: Shape::Spades,
        },
        Card {
            number: Number::Five,
            shape: Shape::Spades,
        },
        Card {
            number: Number::Six,
            shape: Shape::Spades,
        },
        Card {
            number: Number::Seven,
            shape: Shape::Spades,
        },
        Card {
            number: Number::Eight,
            shape: Shape::Spades,
        },
        Card {
            number: Number::Nine,
            shape: Shape::Spades,
        },
        Card {
            number: Number::Ten,
            shape: Shape::Spades,
        },
        Card {
            number: Number::Jack,
            shape: Shape::Spades,
        },
        Card {
            number: Number::Queen,
            shape: Shape::Spades,
        },
        Card {
            number: Number::King,
            shape: Shape::Spades,
        },
        Card {
            number: Number::Ace,
            shape: Shape::Diamonds,
        },
        Card {
            number: Number::Two,
            shape: Shape::Diamonds,
        },
        Card {
            number: Number::Three,
            shape: Shape::Diamonds,
        },
        Card {
            number: Number::Four,
            shape: Shape::Diamonds,
        },
        Card {
            number: Number::Five,
            shape: Shape::Diamonds,
        },
        Card {
            number: Number::Six,
            shape: Shape::Diamonds,
        },
        Card {
            number: Number::Seven,
            shape: Shape::Diamonds,
        },
        Card {
            number: Number::Eight,
            shape: Shape::Diamonds,
        },
        Card {
            number: Number::Nine,
            shape: Shape::Diamonds,
        },
        Card {
            number: Number::Ten,
            shape: Shape::Diamonds,
        },
        Card {
            number: Number::Jack,
            shape: Shape::Diamonds,
        },
        Card {
            number: Number::Queen,
            shape: Shape::Diamonds,
        },
        Card {
            number: Number::King,
            shape: Shape::Diamonds,
        },
    ];
}
