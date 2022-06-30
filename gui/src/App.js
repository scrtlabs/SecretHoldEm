import React from "react";
import * as SecretJS from "secretjs";
import * as bip39 from "bip39";
import { Hand, Table, Card } from "react-casino";
import { Button, Form } from "semantic-ui-react";
import Slider from "rc-slider";
import {
  NotificationContainer,
  NotificationManager,
} from "react-notifications";

import "rc-slider/assets/index.css";
import "semantic-ui-css/semantic.min.css";
import "./App.css";
import "react-notifications/lib/notifications.css";

const PokerSolver = require("pokersolver").Hand;

const nf = new Intl.NumberFormat("en-US", { maximumFractionDigits: 6 });
const codeId = 629;
console.log("Code ID:", codeId);
const refreshTableStateInterval = 2000;

const BIG_BLIND = 1_000_000;
const MAX_TABLE_BIG_BLINDS = 100;
const MIN_TABLE_BIG_BLINDS = 20;

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
  withdrawLoading: false,
  depositLoading: false,
  raiseAmount: 25000,
  depositAmount: 0,
  rematchLoading: false,
  player_a_wants_rematch: false,
  player_b_wants_rematch: false,
  player_a_win_counter: 0,
  player_b_win_counter: 0,
  tie_counter: 0,
  myWalletBalanceUscrt: 0,
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
      "https://api.pulsar.scrttestnet.com",
      myWalletAddress,
      (signBytes) => signingPen.sign(signBytes),
      tx_encryption_seed,
      {
        init: {
          amount: [{ amount: "12500", denom: "uscrt" }],
          gas: "50000",
        },
        exec: {
          amount: [{ amount: "12500", denom: "uscrt" }],
          gas: "50000",
        },
      }
    );

    this.setState({ secretJsClient, myWalletAddress, mnemonic });

    const refreshAllRooms = async () => {
      if (window.location.hash !== "") {
        return;
      }

      try {
        console.log("refreshAllRooms");
        const data = await secretJsClient.getContracts(codeId);

        this.setState({
          all_rooms: data,
        });
      } catch (e) {
        console.log("refreshAllRooms", e);
        NotificationManager.error("refreshAllRooms", e.message, 5000);
      }
    };
    setTimeout(refreshAllRooms, 0);
    setInterval(refreshAllRooms, refreshTableStateInterval);

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

      if (
        JSON.stringify(this.state.my_hand) !== JSON.stringify([{}, {}]) &&
        this.state.stage !== "PreFlop"
      ) {
        // this should work because when switching room (= switching hash location)
        // we set an empty state
        return;
      }

      const secret = +localStorage.getItem(this.state.game_address);
      try {
        console.log("refreshMyHand");

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
        NotificationManager.error("refreshMyHand", e.message, 5000);
      }
    };
    setTimeout(refreshMyHand, 0);
    setInterval(refreshMyHand, refreshTableStateInterval);

    const refreshMyWalletBalance = async () => {
      try {
        console.log("refreshMyWalletBalance");

        const data = await secretJsClient.getAccount(myWalletAddress);

        if (!data) {
          this.setState({
            myWalletBalanceUscrt: 0,
            myWalletBalance: (
              <span>
                (No funds - Go get some{" "}
                <a
                  href="https://faucet.secrettestnet.io"
                  rel="noopener noreferrer"
                  target="_blank"
                >
                  from the faucet
                </a>
                )
              </span>
            ),
          });
        } else {
          this.setState({
            myWalletBalanceUscrt: +data.balance[0].amount,
            myWalletBalance: `(${nf.format(
              +data.balance[0].amount / 1000000
            )} SCRT)`,
          });
        }
      } catch (e) {
        console.log("refreshMyWalletBalance", e);
        NotificationManager.error("refreshMyWalletBalance", e.message, 5000);
      }
    };
    setTimeout(refreshMyWalletBalance, 0);
    setInterval(refreshMyWalletBalance, refreshTableStateInterval * 5);

    const refreshTableState = async () => {
      if (window.location.hash === "") {
        return;
      }

      try {
        console.log("refreshTableState");

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
          community_cards: data.community_cards
            .concat([{}, {}, {}, {}, {}])
            .slice(0, 5),
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
          player_a_wants_rematch: data.player_a_wants_rematch,
          player_b_wants_rematch: data.player_b_wants_rematch,

          player_a_win_counter: data.player_a_win_counter,
          player_b_win_counter: data.player_b_win_counter,
          tie_counter: data.tie_counter,
        });
      } catch (e) {
        console.log("refreshTableState", e);
        NotificationManager.error("refreshTableState", e.message, 5000);
      }
    };

    setTimeout(refreshTableState, 0);
    setInterval(refreshTableState, refreshTableStateInterval);
  }

  async createRoom() {
    this.setState({ createLoading: true });
    try {
      await this.state.secretJsClient.instantiate(
        codeId,
        {
          create_room: { big_blind: BIG_BLIND },
        },
        this.state.new_room_name
      );
    } catch (e) {
      console.log("createRoom", e);
      NotificationManager.error("createRoom", e.message, 5000);
    }
    setTimeout(
      () =>
        this.setState({
          new_room_name: "",
          createLoading: false,
        }),
      refreshTableStateInterval
    );
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
    localStorage.setItem(this.state.game_address, secret);

    try {
      await this.state.secretJsClient.execute(
        this.state.game_address,
        {
          join: { secret },
        },
        "",
        [
          {
            amount: `${this.state.depositAmount}`,
            denom: "uscrt",
          },
        ]
      );
    } catch (e) {
      console.log("join", e);
      NotificationManager.error("join", e.message, 5000);
    }

    setTimeout(
      () => this.setState({ joinLoading: false, depositAmount: 0 }),
      refreshTableStateInterval
    );
  }

  async fold() {
    this.setState({ foldLoading: true });
    try {
      await this.state.secretJsClient.execute(this.state.game_address, {
        fold: {},
      });
    } catch (e) {
      console.log("fold", e);
      NotificationManager.error("fold", e.message, 5000);
    }

    setTimeout(
      () => this.setState({ foldLoading: false }),
      refreshTableStateInterval
    );
  }

  async check() {
    this.setState({ checkLoading: true });
    try {
      await this.state.secretJsClient.execute(this.state.game_address, {
        check: {},
      });
    } catch (e) {
      console.log("check", e);
      NotificationManager.error("check", e.message, 5000);
    }

    setTimeout(
      () => this.setState({ checkLoading: false }),
      refreshTableStateInterval
    );
  }

  async call() {
    this.setState({ callLoading: true });
    try {
      await this.state.secretJsClient.execute(this.state.game_address, {
        call: {},
      });
    } catch (e) {
      console.log("call", e);
      NotificationManager.error("call", e.message, 5000);
    }

    setTimeout(
      () => this.setState({ callLoading: false }),
      refreshTableStateInterval
    );
  }

  async raise() {
    this.setState({ raiseLoading: true });
    try {
      await this.state.secretJsClient.execute(this.state.game_address, {
        raise: { amount: this.state.raiseAmount },
      });
    } catch (e) {
      console.log("raise", e);
      NotificationManager.error("raise", e.message, 5000);
    }
    setTimeout(
      () => this.setState({ raiseLoading: false, raiseAmount: 25000 }),
      refreshTableStateInterval
    );
  }

  async rematch() {
    this.setState({ rematchLoading: true });
    try {
      await this.state.secretJsClient.execute(this.state.game_address, {
        rematch: {},
      });
    } catch (e) {
      console.log("rematch", e);
      NotificationManager.error("rematch", e.message, 5000);
    }
    setTimeout(
      () => this.setState({ rematchLoading: false }),
      refreshTableStateInterval
    );
  }

  async withdraw() {
    this.setState({ withdrawLoading: true });
    try {
      await this.state.secretJsClient.execute(this.state.game_address, {
        withdraw: {},
      });
    } catch (e) {
      console.log("withdraw", e);
      NotificationManager.error("withdraw", e.message, 5000);
    }
    setTimeout(
      () => this.setState({ withdrawLoading: false }),
      refreshTableStateInterval
    );
  }

  async deposit() {
    this.setState({ depositLoading: true });
    try {
      await this.state.secretJsClient.execute(
        this.state.game_address,
        {
          top_up: {},
        },
        "",
        [{ amount: `${this.state.depositAmount}`, denom: "uscrt" }]
      );
    } catch (e) {
      console.log("deposit", e);
      NotificationManager.error("deposit", e.message, 5000);
    }
    setTimeout(
      () => this.setState({ depositLoading: false, depositAmount: 0 }),
      refreshTableStateInterval
    );
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
        wants_rematch: this.state.player_a_wants_rematch,
      };
    }

    if (this.state.myWalletAddress === this.state.player_b) {
      return {
        player: "B",
        address: this.state.player_b,
        bet: this.state.player_b_bet,
        wallet: this.state.player_b_wallet,
        wants_rematch: this.state.player_b_wants_rematch,
      };
    }

    return null;
  }

  getOther() {
    if (!this.state.myWalletAddress) {
      return null;
    }

    if (this.state.myWalletAddress !== this.state.player_a) {
      return {
        player: "A",
        address: this.state.player_a,
        bet: this.state.player_a_bet,
        wallet: this.state.player_a_wallet,
        wants_rematch: this.state.player_a_wants_rematch,
      };
    }

    if (this.state.myWalletAddress !== this.state.player_b) {
      return {
        player: "B",
        address: this.state.player_b,
        bet: this.state.player_b_bet,
        wallet: this.state.player_b_wallet,
        wants_rematch: this.state.player_b_wants_rematch,
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
              <div
                style={{
                  position: "relative",
                  zIndex: 9999,
                }}
              >
                You: {this.state.myWalletAddress} {this.state.myWalletBalance}
              </div>
            </div>
            <div
              style={{
                position: "relative",
                zIndex: 9999,
              }}
            >
              <div
                style={{
                  textAlign: "center",
                }}
              >
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
                  <thead>
                    <tr>
                      <th>Room Name</th>
                      <th>Address</th>
                    </tr>
                  </thead>
                  <tbody>
                    {this.state.all_rooms.map((r, i) => (
                      <tr key={i}>
                        <td>{r.label}</td>
                        <td>
                          <a href={"#" + r.address}>{r.address}</a>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </center>
            </div>
          </Table>
          <NotificationContainer />
        </div>
      );
    }

    const handA = this.state.player_a_hand
      .concat(this.state.community_cards)
      .map(stateCardToPokerSoverCard)
      .filter((x) => x);
    let rankHandA = "Unknown";
    if (handA.length > 5 || (this.getMe() && this.getMe().player === "A")) {
      try {
        const solve = PokerSolver.solve(handA);
        rankHandA = solve.descr;
      } catch (e) {}
    }

    const handB = this.state.player_b_hand
      .concat(this.state.community_cards)
      .map(stateCardToPokerSoverCard)
      .filter((x) => x);
    let rankHandB = "Unknown";
    if (handB.length > 5 || (this.getMe() && this.getMe().player === "B")) {
      try {
        const solve = PokerSolver.solve(handB);
        rankHandB = solve.descr;
      } catch (e) {}
    }

    let stage = this.state.stage;
    if (stage.includes("EndedWinner")) {
      const winner = stage.replace("EndedWinner", "");
      stage = (
        <span>
          <div>
            <b>Player {winner} Wins!</b>
          </div>

          {typeof this.state.last_play === "string" &&
          !this.state.last_play.includes("fold") ? (
            rankHandA !== rankHandB ? (
              <div>
                <b>{winner === "A" ? rankHandA : rankHandB}</b> vs. a lousy{" "}
                <b>{winner === "A" ? rankHandB : rankHandA}</b>
              </div>
            ) : (
              <div>
                Both with <b>{rankHandA}</b>, {winner} won with a kicker!
              </div>
            )
          ) : null}
        </span>
      );
    } else if (stage.includes("EndedDraw")) {
      stage = `It's a Tie of ${rankHandA}!`;
    } else if (stage === "WaitingForPlayersToJoin") {
      const isLoading =
        this.state.joinLoading ||
        !!this.getMe() ||
        !!(
          this.state.myWalletBalance &&
          typeof this.state.myWalletBalance === "string" &&
          !this.state.myWalletBalance.includes("SCRT")
        );
      stage = (
        <span>
          <div>Waiting for players</div>
          <div>
            <Button
              loading={isLoading}
              disabled={
                isLoading ||
                this.state.myWalletBalanceUscrt === 0 ||
                this.state.depositAmount === 0
              }
              onClick={this.joinRoom.bind(this)}
            >
              Join and Deposit
              {` (${nf.format(
                this.state.depositAmount / 1000000
              )} SCRT = ${nf.format(this.state.depositAmount)} credits)`}
            </Button>
            <center hidden={isLoading || this.state.myWalletBalanceUscrt === 0}>
              <Slider
                style={{ width: "400px" }}
                min={
                  this.state.myWalletBalanceUscrt === 0
                    ? 0
                    : MIN_TABLE_BIG_BLINDS * BIG_BLIND
                }
                value={this.state.depositAmount}
                max={Math.min(
                  this.state.myWalletBalanceUscrt,
                  MAX_TABLE_BIG_BLINDS * BIG_BLIND
                )}
                onChange={(v) => this.setState({ depositAmount: v })}
              />
            </center>
          </div>
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
    if (typeof this.state.last_play === "string") {
      if (this.state.last_play.includes("fold")) {
        lastPlay = this.state.last_play;
      } else if (this.state.last_play.includes("raised")) {
        try {
          const amount = +this.state.last_play.match(/\d+/g)[0];
          lastPlay = this.state.last_play.replace(
            `${amount}`,
            nf.format(amount)
          );
        } catch (e) {}
      }
    }

    let rematch = null;
    if (
      typeof this.state.stage === "string" &&
      this.state.stage.includes("Ended")
    ) {
      rematch = (
        <div>
          {this.getMe() && this.getOther() ? (
            <Button
              loading={this.state.rematchLoading || this.getMe().wants_rematch}
              onClick={this.rematch.bind(this)}
              disabled={
                this.state.rematchLoading ||
                this.getMe().wallet === 0 ||
                this.getOther().wallet === 0 ||
                this.getMe().wants_rematch
              }
            >
              Rematch!
            </Button>
          ) : null}
          {this.state.player_a_wants_rematch ? (
            <div style={{ padding: 10 }}>Rematch: Waiting for player B.</div>
          ) : null}
          {this.state.player_b_wants_rematch ? (
            <div style={{ padding: 10 }}>Rematch: Waiting for player A.</div>
          ) : null}
        </div>
      );
    }

    let room = "";
    if (this.state.game_address) {
      room = "Room: " + this.state.game_address;
    }

    let minDeposit = 0;
    let maxDeposit = 0;
    if (this.getMe()) {
      const alreadyInside = this.getMe().wallet + this.getMe().bet;

      maxDeposit = BIG_BLIND * MAX_TABLE_BIG_BLINDS - alreadyInside;
      maxDeposit = Math.max(maxDeposit, 0);

      minDeposit = BIG_BLIND * MIN_TABLE_BIG_BLINDS - alreadyInside;
      minDeposit = Math.max(minDeposit, 0);
    }

    return (
      <div style={{ color: "white" }}>
        <Table>
          {/* wallet + scoreboard */}
          <div
            style={{
              position: "absolute",
              top: 0,
              left: 0,
              padding: 10,
            }}
          >
            <div style={{ width: "700px" }}>
              <div
                style={{
                  position: "relative",
                  zIndex: 9999,
                }}
              >
                You: {this.state.myWalletAddress} {this.state.myWalletBalance}
              </div>
              <div>
                <Button
                  loading={this.state.withdrawLoading}
                  onClick={this.withdraw.bind(this)}
                  disabled={
                    this.state.withdrawLoading ||
                    !this.getMe() ||
                    this.getMe().wallet === 0
                  }
                >
                  {!this.state.stage.includes("Ended") &&
                  !this.state.stage.includes("Waiting")
                    ? "Fold + "
                    : ""}
                  Withdraw
                  {this.getMe()
                    ? ` (${nf.format(this.getMe().wallet / 1000000)} SCRT)`
                    : ""}
                </Button>
                <Button
                  loading={this.state.depositLoading}
                  onClick={this.deposit.bind(this)}
                  disabled={
                    this.state.depositLoading ||
                    this.state.depositAmount === 0 ||
                    !this.getMe()
                  }
                >
                  Deposit
                  {this.getMe()
                    ? ` (${nf.format(
                        this.state.depositAmount / 1000000
                      )} SCRT = ${nf.format(this.state.depositAmount)} credits)`
                    : ""}
                </Button>
                <div hidden={!this.getMe() || minDeposit >= maxDeposit}>
                  <Slider
                    style={{ width: "400px" }}
                    min={minDeposit}
                    value={this.state.depositAmount}
                    max={maxDeposit}
                    onChange={(v) => this.setState({ depositAmount: v })}
                  />
                </div>
              </div>
            </div>

            <div
              style={{
                marginTop: 30,
                position: "relative",
                zIndex: 9999,
              }}
            >
              <table>
                <thead>
                  <tr>
                    <td>
                      <center>Wins:</center>
                    </td>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <th>Player A</th>
                    <td>{this.state.player_a_win_counter}</td>
                  </tr>
                  <tr>
                    <th>Player B</th>
                    <td>{this.state.player_b_win_counter}</td>
                  </tr>
                  <tr>
                    <th>Ties</th>
                    <td>{this.state.tie_counter}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
          {/* return to lobby */}
          <div
            style={{
              position: "absolute",
              top: 0,
              right: 15,
              padding: 10,
            }}
          >
            <a
              style={{
                position: "relative",
                zIndex: 9999,
              }}
              href="/#"
            >
              Return to lobby
            </a>
          </div>
          {/* community cards */}
          <div
            style={{ position: "absolute", width: "100%", textAlign: "center" }}
          >
            <div
              style={{
                position: "relative",
                zIndex: 9999,
              }}
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
                  B's Total Bet: {nf.format(this.state.player_b_bet)}
                </span>
                <span style={{ marginLeft: 125 }}>
                  A's Total Bet: {nf.format(this.state.player_a_bet)}
                </span>
              </div>
              <div
                hidden={!lastPlay}
                style={{ padding: 35, textAlign: "center" }}
              >
                {lastPlay}
              </div>
              <div
                hidden={!rematch}
                style={{ padding: 35, textAlign: "center" }}
              >
                {rematch}
              </div>
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
              <div className="ui active inline loader" />
            ) : null}
            <div>
              Player A
              {this.state.player_a === this.state.myWalletAddress
                ? " (You)"
                : ""}
            </div>
            <div>
              Hand: <b>{rankHandA}</b>
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
              zIndex: 999,
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
                this.state.checkLoading ||
                this.state.raiseAmount <= 0
              }
            >
              {this.getMe() &&
              this.getOther() &&
              this.state.raiseAmount + this.getOther().bet ===
                this.getMe().wallet + this.getMe().bet
                ? "All in!"
                : `${
                    this.getMe() &&
                    this.getOther() &&
                    this.getMe().bet === 0 &&
                    this.getOther().bet === 0
                      ? "Bet"
                      : "Raise"
                  } ${nf.format(this.state.raiseAmount)}`}
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
            <center>
              <div style={{ padding: 10, width: "300px" }}>
                <Slider
                  min={0}
                  value={this.state.raiseAmount}
                  max={
                    this.getOther() && this.getMe()
                      ? Math.min(
                          this.getOther().wallet,
                          this.getMe().wallet -
                            (this.getOther().bet - this.getMe().bet)
                        )
                      : 0
                  }
                  onChange={(v) => this.setState({ raiseAmount: v })}
                />
              </div>
            </center>
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
              <div className="ui active inline loader" />
            ) : null}
            <div>
              Player B{" "}
              {this.state.player_b === this.state.myWalletAddress
                ? " (You)"
                : ""}
            </div>
            <div>
              Hand: <b>{rankHandB}</b>
            </div>
            <div>Credits left: {nf.format(this.state.player_b_wallet)}</div>
            <div>{this.state.player_b}</div>
          </div>

          <Hand
            style={{ position: "absolute", left: "23vw" }}
            cards={this.state.player_b_hand.map((c) => stateCardToReactCard(c))}
          />
        </Table>
        <NotificationContainer />
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

function stateCardToPokerSoverCard(c) {
  if (!c.value || !c.suit) {
    return null;
  }

  let type = {
    Spade: "s",
    Club: "c",
    Heart: "h",
    Diamond: "d",
  }[c.suit];

  let rank = {
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

  return rank + type;
}

export default App;
