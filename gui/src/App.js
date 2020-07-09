import React from "react";
import * as SecretJS from "secretjs";
import * as bip39 from "bip39";
import { Hand, Table, Card } from "react-casino";

import { Slider } from "react-semantic-ui-range";
import { Button, Form } from "semantic-ui-react";
import "semantic-ui-css/semantic.min.css";

const nf = new Intl.NumberFormat();
const codeId = 12;

const emptyState = {
  game_address: "",
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
  createLoading: false,
  joinLoading: false,
  checkLoading: false,
  callLoading: false,
  raiseLoading: false,
  raiseAmount: 10000,
};

class App extends React.Component {
  constructor(props) {
    super(props);

    this.state = Object.assign({}, emptyState, {
      game_address: window.location.hash.replace("#", ""),
    });
  }

  async componentDidMount() {
    window.onhashchange = async () => {
      this.setState(
        Object.assign({}, emptyState, {
          game_address: window.location.hash.replace("#", ""),
        })
      );
    };

    let mnemonic = localStorage.getItem("mnemonic");
    if (!mnemonic) {
      mnemonic = bip39.generateMnemonic();
      localStorage.setItem("mnemonic", mnemonic);
    }

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
      if (window.location.hash !== "") {
        return;
      }

      try {
        const data = await secretJsClient.getContracts(codeId);

        this.setState({
          all_rooms: data,
        });
      } catch (e) {
        console.log("refreshAllRooms", e);
      }
    };
    setTimeout(refreshAllRooms, 0);
    setInterval(refreshAllRooms, 200);

    const refreshMyHand = async () => {
      if (window.location.hash === "") {
        return;
      }

      if (!this.state.player_a || !this.state.player_b) {
        return;
      }

      if (
        this.state.player_a !== this.state.myWalletAddress &&
        this.state.player_b !== this.state.myWalletAddress
      ) {
        return;
      }

      if (JSON.stringify(this.state.my_hand) !== JSON.stringify([{}, {}])) {
        // this should work because when switching room (= switching hash location)
        // we set an empty state
        return;
      }

      const secret = +localStorage.getItem(this.state.game_address);
      try {
        const data = await secretJsClient.queryContractSmart(
          this.state.game_address,
          { get_my_hand: { secret } }
        );

        this.setState({
          my_hand: data,
        });

        if (this.state.myWalletAddress === this.state.player_a) {
          this.setState({
            player_a_hand: this.state.my_hand,
          });
        } else if (this.state.myWalletAddress === this.state.player_b) {
          this.setState({
            player_b_hand: this.state.my_hand,
          });
        }
      } catch (e) {
        console.log("refreshMyHand", e);
      }
    };
    setTimeout(refreshMyHand, 0);
    setInterval(refreshMyHand, 200);

    const refreshMyWalletBalance = async () => {
      try {
        const data = await secretJsClient.getAccount(myWalletAddress);

        if (!data) {
          this.setState({
            myWalletBalance: (
              <span>
                (No funds - Go get some at{" "}
                <a
                  href="https://faucet.testnet.enigma.co"
                  rel="noopener noreferrer"
                  target="_blank"
                >
                  https://faucet.testnet.enigma.co
                </a>
                )
              </span>
            ),
          });
        } else {
          this.setState({
            myWalletBalance: `(${nf.format(
              +data.balance[0].amount / 1000000
            )} SCRT)`,
          });
        }
      } catch (e) {
        console.log("refreshMyWalletBalance", e);
      }
    };
    setTimeout(refreshMyWalletBalance, 0);
    setInterval(refreshMyWalletBalance, 2500);

    const refreshTableState = async () => {
      if (window.location.hash === "") {
        return;
      }

      if (this.state.stage.includes("Ended")) {
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

        if (this.state.myWalletAddress === data.player_a) {
          this.setState({
            player_a_hand: this.state.my_hand,
            player_b_hand: data.player_b_hand,
          });
        } else if (this.state.myWalletAddress === data.player_b) {
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
          last_play: data.last_play,
        });
      } catch (e) {
        console.log("refreshTableState", e);
      }
    };

    setTimeout(refreshTableState, 0);
    setInterval(refreshTableState, 200);
  }

  async createRoom() {
    this.setState({ createLoading: true });
    try {
      await this.state.secretJsClient.instantiate(
        codeId,
        {},
        this.state.new_room_name
      );
    } catch (e) {
      console.log("createRoom", e);
    }
    this.setState({ new_room_name: "", createLoading: false });
  }

  async joinRoom() {
    if (!this.state.game_address) {
      // ah?
      return;
    }

    this.setState({ joinLoading: true });

    let secret = +localStorage.getItem(this.state.game_address);
    if (!secret) {
      const seed = SecretJS.EnigmaUtils.GenerateNewSeed();
      secret = Buffer.from(seed.slice(0, 8)).readUInt32BE(0); // 64 bit
    }

    try {
      await this.state.secretJsClient.execute(this.state.game_address, {
        join: { secret },
      });
    } catch (e) {
      console.log("join", e);
    }

    localStorage.setItem(this.state.game_address, secret);

    this.setState({ joinLoading: false });
  }

  async fold() {
    this.setState({ foldLoading: true });
    try {
      await this.state.secretJsClient.execute(this.state.game_address, {
        fold: {},
      });
    } catch (e) {
      console.log("fold", e);
    }
    this.setState({ foldLoading: false });
  }

  async check() {
    this.setState({ checkLoading: true });
    try {
      await this.state.secretJsClient.execute(this.state.game_address, {
        check: {},
      });
    } catch (e) {
      console.log("check", e);
    }
    this.setState({ checkLoading: false });
  }

  async call() {
    this.setState({ callLoading: true });
    try {
      await this.state.secretJsClient.execute(this.state.game_address, {
        call: {},
      });
    } catch (e) {
      console.log("call", e);
    }
    this.setState({ callLoading: false });
  }

  async raise() {
    this.setState({ raiseLoading: true });
    try {
      await this.state.secretJsClient.execute(this.state.game_address, {
        raise: { amount: 10000 },
      });
    } catch (e) {
      console.log("raise", e);
    }
    this.setState({ raiseLoading: false });
  }

  getMe() {
    if (!this.state.myWalletAddress) {
      return null;
    }

    if (this.state.myWalletAddress === this.state.player_a) {
      return {
        player: "A",
        address: this.state.player_a,
        bet: this.state.player_a_bet,
        wallet: this.state.player_a_wallet,
      };
    }

    if (this.state.myWalletAddress === this.state.player_b) {
      return {
        player: "B",
        address: this.state.player_b,
        bet: this.state.player_b_bet,
        wallet: this.state.player_b_wallet,
      };
    }

    return null;
  }

  render() {
    if (window.location.hash === "") {
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
            <div style={{ textAlign: "center" }}>
              <Form.Input
                placeholder="Room name"
                value={this.state.new_room_name}
                onChange={(_, { value }) =>
                  this.setState({ new_room_name: value })
                }
              />
              <Button
                loading={this.state.createLoading}
                disabled={this.state.createLoading}
                onClick={this.createRoom.bind(this)}
              >
                Create!
              </Button>
            </div>
            <br />
            <center>
              <table>
                <tr>
                  <th>Name</th>
                  <th>Address</th>
                </tr>
                {this.state.all_rooms.map((r, i) => (
                  <tr key={i}>
                    <td>{r.label}</td>
                    <td>
                      <a href={"#" + r.address}>{r.address}</a>
                    </td>
                  </tr>
                ))}
              </table>
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
          <Button
            loading={this.state.joinLoading}
            disabled={
              this.state.joinLoading ||
              this.getMe() ||
              (this.state.myWalletBalance &&
                typeof this.state.myWalletBalance === "string" &&
                !this.state.myWalletBalance.includes("SCRT"))
            }
            onClick={this.joinRoom.bind(this)}
          >
            Join
          </Button>
        </span>
      );
    } else if (stage) {
      stage += " betting round";
    }

    let turn = "Player A";
    let turnDirection = "->";
    let lastPlay = this.state.last_play || "";
    if (this.state.turn === this.state.player_b) {
      turn = "Player B";
      turnDirection = "<-";
    }
    turn = "Turn: " + turn;
    if (
      !this.state.stage ||
      !this.state.turn ||
      this.state.stage.includes("Ended") ||
      this.state.stage.includes("Waiting")
    ) {
      turn = "";
      turnDirection = "";
      lastPlay = "";
    }
    if (
      typeof this.state.last_play === "string" &&
      this.state.last_play.includes("fold")
    ) {
      lastPlay = this.state.last_play;
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
          <div
            style={{ position: "absolute", width: "100%", textAlign: "center" }}
          >
            <div>{room}</div>
            <div>{stage}</div>
            <div>{turn}</div>
            <div>{turnDirection}</div>
            <br />
            {this.state.community_cards.map((c, i) =>
              stateCardToReactCard(c, true, i)
            )}
            <div style={{ padding: 35, textAlign: "center" }}>
              <span style={{ marginRight: 125 }}>
                A Total Bet: {nf.format(this.state.player_b_bet)}
              </span>
              <span style={{ marginLeft: 125 }}>
                B Total Bet: {nf.format(this.state.player_a_bet)}
              </span>
            </div>
            <div
              hidden={!lastPlay}
              style={{ padding: 35, textAlign: "center" }}
            >
              {lastPlay}
            </div>
          </div>
          {/* player a */}
          <div
            style={{
              position: "absolute",
              bottom: 0,
              right: 0,
              padding: 10,
              textAlign: "center",
            }}
          >
            {turn.includes("Player A") ? (
              <div class="ui active inline loader" />
            ) : null}
            <div>
              Player A
              {this.state.player_a === this.state.myWalletAddress
                ? " (You)"
                : ""}
            </div>
            <div>Credits left: {nf.format(this.state.player_a_wallet)}</div>
            <div>{this.state.player_a}</div>
          </div>
          <Hand
            style={{ position: "absolute", right: "35vw" }}
            cards={this.state.player_a_hand.map((c) => stateCardToReactCard(c))}
          />
          {/* controls */}
          <div
            style={{
              position: "fixed",
              bottom: 0,
              padding: 10,
              width: "100%",
              textAlign: "center",
            }}
            hidden={
              !this.getMe() ||
              this.state.stage.includes("Ended") ||
              this.state.stage.includes("Waiting")
            }
          >
            <Button
              loading={this.state.checkLoading}
              onClick={this.check.bind(this)}
              disabled={
                this.state.player_a_bet !== this.state.player_b_bet ||
                !this.state.turn ||
                this.state.turn !== this.state.myWalletAddress ||
                this.state.stage.includes("Ended") ||
                this.state.stage.includes("Waiting") ||
                this.state.callLoading ||
                this.state.raiseLoading ||
                this.state.foldLoading ||
                this.state.checkLoading
              }
            >
              Check
            </Button>
            <Button
              loading={this.state.callLoading}
              onClick={this.call.bind(this)}
              disabled={
                this.state.player_a_bet === this.state.player_b_bet ||
                !this.state.turn ||
                this.state.turn !== this.state.myWalletAddress ||
                this.state.stage.includes("Ended") ||
                this.state.stage.includes("Waiting") ||
                this.state.callLoading ||
                this.state.raiseLoading ||
                this.state.foldLoading ||
                this.state.checkLoading
              }
            >
              Call
            </Button>
            <Button
              loading={this.state.raiseLoading}
              onClick={this.raise.bind(this)}
              disabled={
                !this.state.turn ||
                this.state.turn !== this.state.myWalletAddress ||
                this.state.stage.includes("Ended") ||
                this.state.stage.includes("Waiting") ||
                this.state.callLoading ||
                this.state.raiseLoading ||
                this.state.foldLoading ||
                this.state.checkLoading
              }
            >
              Raise
            </Button>
            <Button
              loading={this.state.foldLoading}
              onClick={this.fold.bind(this)}
              disabled={
                !this.state.turn ||
                this.state.turn !== this.state.myWalletAddress ||
                this.state.stage.includes("Ended") ||
                this.state.stage.includes("Waiting") ||
                this.state.callLoading ||
                this.state.raiseLoading ||
                this.state.foldLoading ||
                this.state.checkLoading
              }
            >
              Fold
            </Button>
            {/* <Slider
              value={10000}
              color="red"
              settings={{
                start: 2,
                min: 0,
                max: 10,
                step: 1,
                onChange: (value) => {
                  this.setState({ raiseAmount: value });
                },
              }}
            /> */}
          </div>
          {/* player b */}
          <div
            style={{
              position: "absolute",
              bottom: 0,
              left: 0,
              padding: 10,
              textAlign: "center",
            }}
          >
            {turn.includes("Player B") ? (
              <div class="ui active inline loader" />
            ) : null}
            <div>
              Player B{" "}
              {this.state.player_b === this.state.myWalletAddress
                ? " (You)"
                : ""}
            </div>
            <div>Credits left: {nf.format(this.state.player_b_wallet)}</div>
            <div>{this.state.player_b}</div>
          </div>

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
  if (!c.value || !c.suit) {
    if (component) {
      return <Card key={index} />;
    } else {
      return {};
    }
  }

  let suit = {
    Spade: "S",
    Club: "C",
    Heart: "H",
    Diamond: "D",
  }[c.suit];

  let face = {
    Two: "2",
    Three: "3",
    Four: "4",
    Five: "5",
    Six: "6",
    Seven: "7",
    Eight: "8",
    Nine: "9",
    Ten: "T",
    Jack: "J",
    Queen: "Q",
    King: "K",
    Ace: "A",
  }[c.value];

  if (component) {
    return <Card key={index} face={face} suit={suit} />;
  } else {
    return { face, suit };
  }
}

export default App;
