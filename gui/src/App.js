import React from "react";
import * as SecretJS from "secretjs";
import * as bip39 from "bip39";
import { Hand, Table, Card } from "react-casino";

import { Button, Form } from "semantic-ui-react";
import "semantic-ui-css/semantic.min.css";

const nf = new Intl.NumberFormat();
const codeId = 9;

const emptyState = {
  game_address: window.location.hash.replace("#", ""),
  all_rooms: [],
  community_cards: [],
  my_hand: [{}, {}],
  player_a_hand: [{}, {}],
  player_b_hand: [{}, {}],
  player_a: "",
  player_a_bet: 0,
  player_a_wallet: 0,
  player_b: "",
  player_b_bet: 0,
  player_b_wallet: 0,
  stage: "",
  turn: "",
  new_room_name: "",
};

class App extends React.Component {
  constructor(props) {
    super(props);

    this.state = Object.assign({}, emptyState);
  }

  async componentDidMount() {
    window.onhashchange = async () => {
      this.setState({
        game_address: window.location.hash.replace("#", ""),
      });

      if (window.location.hash === "") {
        this.setState(Object.assign({}, emptyState));
        return;
      }

      try {
        const data = await secretJsClient.queryContractSmart(
          this.state.game_address,
          { get_my_hand: { secret: 234 } }
        );

        this.setState({
          my_hand: data,
        });
      } catch (e) {}
    };

    let mnemonic = localStorage.getItem("mnemonic");
    if (!mnemonic) {
      mnemonic = bip39.generateMnemonic();
      localStorage.setItem("mnemonic", mnemonic);
    }
    mnemonic =
      "web use october receive enforce desk stick arena toast vacuum swear spike about company dragon amused various glide ball maze anxiety lake umbrella light";

    let tx_encryption_seed = localStorage.getItem("tx_encryption_seed");
    if (tx_encryption_seed) {
      tx_encryption_seed = Uint8Array.from(
        JSON.parse(`[${tx_encryption_seed}]`)
      );
    } else {
      tx_encryption_seed = SecretJS.EnigmaUtils.GenerateNewSeed();
      localStorage.setItem("tx_encryption_seed", tx_encryption_seed);
    }

    const signingPen = await SecretJS.Secp256k1Pen.fromMnemonic(mnemonic);
    const myWalletAddress = SecretJS.pubkeyToAddress(
      SecretJS.encodeSecp256k1Pubkey(signingPen.pubkey),
      "secret"
    );
    const secretJsClient = new SecretJS.SigningCosmWasmClient(
      "https://bootstrap.int.testnet.enigma.co",
      myWalletAddress,
      (signBytes) => signingPen.sign(signBytes),
      tx_encryption_seed,
      {
        init: {
          amount: [{ amount: "0", denom: "uscrt" }],
          gas: "500000",
        },
        exec: {
          amount: [{ amount: "0", denom: "uscrt" }],
          gas: "500000",
        },
      }
    );

    this.setState({ secretJsClient, myWalletAddress, mnemonic });

    const refreshAllRooms = async () => {
      const data = await secretJsClient.getContracts(codeId);

      this.setState({
        all_rooms: data,
      });
    };
    setTimeout(refreshAllRooms, 0);
    setInterval(refreshAllRooms, 1000);

    setTimeout(async () => {
      if (window.location.hash === "") {
        return;
      }

      try {
        const data = await secretJsClient.queryContractSmart(
          this.state.game_address,
          { get_my_hand: { secret: 234 } }
        );

        this.setState({
          my_hand: data,
        });
      } catch (e) {}
    }, 0);

    const refreshMyWalletBalance = async () => {
      const data = await secretJsClient.getAccount(myWalletAddress);

      if (!data) {
        this.setState({ myWalletBalance: "(No funds)" });
      } else {
        this.setState({
          myWalletBalance: `(${nf.format(
            +data.balance[0].amount / 1000000
          )} SCRT)`,
        });
      }
    };
    setTimeout(refreshMyWalletBalance, 0);
    setInterval(refreshMyWalletBalance, 10000);

    const refreshTableState = async () => {
      if (window.location.hash === "") {
        return;
      }

      try {
        const data = await secretJsClient.queryContractSmart(
          this.state.game_address,
          { get_public_data: {} }
        );

        if (data.player_a_hand.length === 0) {
          data.player_a_hand = [{}, {}];
        }
        if (data.player_b_hand.length === 0) {
          data.player_b_hand = [{}, {}];
        }

        if (myWalletAddress === data.player_a) {
          this.setState({
            player_a_hand: this.state.my_hand,
            player_b_hand: data.player_b_hand,
          });
        }
        if (myWalletAddress === data.player_b) {
          this.setState({
            player_a_hand: data.player_a_hand,
            player_b_hand: this.state.my_hand,
          });
        } else {
          this.setState({
            player_a_hand: data.player_a_hand,
            player_b_hand: data.player_b_hand,
          });
        }

        this.setState({
          community_cards: data.community_cards,
          player_a: data.player_a,
          player_a_bet: data.player_a_bet,
          player_a_wallet: data.player_a_wallet,
          player_b: data.player_b,
          player_b_bet: data.player_b_bet,
          player_b_wallet: data.player_b_wallet,
          stage: data.stage,
          starter: data.starter,
          turn: data.turn,
        });
      } catch (e) {
        // Probably table is only after init
      }
    };

    setTimeout(refreshTableState, 0);
    setInterval(refreshTableState, 3000);
  }
  async createRoom() {
    await this.state.secretJsClient.instantiate(
      codeId,
      {},
      this.state.new_room_name
    );
    this.setState({ new_room_name: "" });
  }
  render() {
    if (window.location.hash === "") {
      return (
        <div style={{ color: "white" }}>
          <Table>
            <center>
              <div>
                <Form.Input
                  value={this.state.new_room_name}
                  onChange={(_, { value }) =>
                    this.setState({ new_room_name: value })
                  }
                />
                <Button onClick={this.createRoom.bind(this)}>Create!</Button>
              </div>
              <br />
              <div>All rooms</div>
              {this.state.all_rooms.map((r, i) => (
                <div key={i}>
                  {r.label}: <a href={"#" + r.address}>{r.address}</a>
                </div>
              ))}
            </center>
          </Table>
        </div>
      );
    }

    let stage = this.state.stage;
    if (stage.includes("EndedWinner")) {
      stage = stage.replace("EndedWinner", "");
      stage = `Player ${stage} Wins!`;
    } else if (stage.includes("EndedDraw")) {
      stage = "It's a Tie!";
    } else if (stage === "WaitingForPlayersToJoin") {
      stage = (
        <span>
          <div>Waiting for players</div>
          <Button>Join</Button>
        </span>
      );
    } else if (stage) {
      stage += " betting round";
    }

    let turn = "Player A";
    let turnDirection = "->";
    if (this.state.turn === this.state.player_b) {
      turn = "Player B";
      turnDirection = "<-";
    }
    turn = "Turn: " + turn;
    if (!this.state.stage || this.state.stage.includes("Ended")) {
      turn = "";
      turnDirection = "";
    }
    if (!this.state.turn) {
      turn = "";
      turnDirection = "";
    }

    let room = "";
    if (this.state.game_address) {
      room = "Room: " + this.state.game_address;
    }

    return (
      <div style={{ color: "white" }}>
        <Table>
          {/* wallet */}
          <div
            style={{
              position: "absolute",
              top: 0,
              left: 0,
              padding: 10,
            }}
          >
            <div>
              You: {this.state.myWalletAddress} {this.state.myWalletBalance}
            </div>
          </div>
          {/* return to loby */}
          <div
            style={{
              position: "absolute",
              top: 0,
              right: 0,
              padding: 10,
            }}
          >
            <a href="/#">Return to loby</a>
          </div>
          {/* community cards */}
          <div style={{ position: "absolute", left: "35vw" }}>
            <center>
              <div>{room}</div>
              <div>{stage}</div>
              <div>{turn}</div>
              <div>{turnDirection}</div>
            </center>

            <br />
            {this.state.community_cards.map((c, i) =>
              stateCardToReactCard(c, true, i)
            )}
            <center>
              <div style={{ padding: 35 }}>
                <span style={{ marginRight: 250 }}>
                  Total Bet: {nf.format(this.state.player_a_bet)}
                </span>
                <span>Total Bet: {nf.format(this.state.player_a_bet)}</span>
              </div>
            </center>
          </div>
          {/* player a */}
          <center>
            <div
              style={{
                position: "absolute",
                bottom: 0,
                right: 0,
                padding: 10,
              }}
            >
              <div>
                Player A
                {this.state.player_a === this.state.myWalletAddress
                  ? " (You)"
                  : ""}
              </div>
              <div>Credits left: {nf.format(this.state.player_a_wallet)}</div>
              <div>{this.state.player_a}</div>
            </div>
          </center>
          <Hand
            style={{ position: "absolute", right: "35vw" }}
            cards={this.state.player_a_hand.map((c) => stateCardToReactCard(c))}
          />
          {/* controls */}
          <center>
            <div
              style={{
                position: "absolute",
                bottom: 0,
                left: 600,
                padding: 10,
              }}
            >
              <Button
                disabled={
                  !this.state.turn ||
                  this.state.turn !== this.state.myWalletAddress
                }
              >
                Check
              </Button>
              <Button
                disabled={
                  !this.state.turn ||
                  this.state.turn !== this.state.myWalletAddress
                }
              >
                Call
              </Button>
              <Button
                disabled={
                  !this.state.turn ||
                  this.state.turn !== this.state.myWalletAddress
                }
              >
                Raise
              </Button>
              <Button
                disabled={
                  !this.state.turn ||
                  this.state.turn !== this.state.myWalletAddress
                }
              >
                Fold
              </Button>
            </div>
          </center>
          {/* player b */}
          <center>
            <div
              style={{
                position: "absolute",
                bottom: 0,
                left: 0,
                padding: 10,
              }}
            >
              <div>
                Player B{" "}
                {this.state.player_b === this.state.myWalletAddress
                  ? " (You)"
                  : ""}
              </div>
              <div>Credits left: {nf.format(this.state.player_b_wallet)}</div>
              <div>{this.state.player_b}</div>
            </div>
          </center>

          <Hand
            style={{ position: "absolute", left: "23vw" }}
            cards={this.state.player_b_hand.map((c) => stateCardToReactCard(c))}
          />
        </Table>
      </div>
    );
  }
}

function stateCardToReactCard(c, component = false, index) {
  let suit = c.suit;
  let value = c.value;

  if (!c.value || !c.suit) {
    if (component) {
      return <Card key={index} />;
    } else {
      return {};
    }
  }

  suit = suit[0];
  let face;
  if (value === "Two") {
    face = "2";
  } else if (value === "Three") {
    face = "3";
  } else if (value === "Four") {
    face = "4";
  } else if (value === "Five") {
    face = "5";
  } else if (value === "Six") {
    face = "6";
  } else if (value === "Seven") {
    face = "7";
  } else if (value === "Eight") {
    face = "8";
  } else if (value === "Nine") {
    face = "9";
  } else if (value === "Ten") {
    face = "10";
  } else {
    face = value[0];
  }

  if (component) {
    return <Card key={index} face={face} suit={suit} />;
  } else {
    return { face, suit };
  }
}

export default App;
