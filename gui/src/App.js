import React from "react";
// import { Form, TextArea, Input, Icon } from "semantic-ui-react";
import * as SecretJS from "secretjs";
import * as bip39 from "bip39";
import { Hand, Table, Card } from "react-casino";

import "semantic-ui-css/semantic.min.css";
import "./App.css";

class App extends React.Component {
  constructor(props) {
    super(props);
    this.state = {
      community_cards: [],
      my_hand: [{}, {}],
      player_a_hand: [{}, {}],
      player_b_hand: [{}, {}],
    };
  }

  async componentDidMount() {
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

    setTimeout(async () => {
      const data = await secretJsClient.queryContractSmart(
        "secret1xzlgeyuuyqje79ma6vllregprkmgwgavk8y798",
        { get_my_hand: { secret: 123 } }
      );

      this.setState({
        my_hand: data,
      });
    }, 0);

    const refresh = async () => {
      const data = await secretJsClient.queryContractSmart(
        "secret1xzlgeyuuyqje79ma6vllregprkmgwgavk8y798",
        { get_public_data: {} }
      );

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
    };

    setTimeout(refresh, 0);
    setInterval(refresh, 3000);
  }

  render() {
    return (
      <div style={{ color: "white" }}>
        <center>
          <Table>
            <div style={{ position: "absolute", left: "35vw" }}>
              <div>{this.state.stage ? "Stage: " + this.state.stage : ""}</div>
              <div>{this.state.turn ? "Turn: " + this.state.turn : ""}</div>
              <br />
              {this.state.community_cards.map((c) =>
                stateCardToReactCard(c, true)
              )}
            </div>
            {/* player a */}
            <div
              style={{ position: "absolute", bottom: 0, right: 0, padding: 10 }}
            >
              <div>
                Player A
                {this.state.player_a === this.myWalletAddress ? " (You)" : ""}
              </div>
              <div>{this.state.player_a ? this.state.player_a : ""}</div>
            </div>
            <Hand
              style={{ position: "absolute", right: "35vw" }}
              cards={this.state.player_a_hand.map((c) =>
                stateCardToReactCard(c)
              )}
            />
            {/* player b */}
            <div
              style={{ position: "absolute", bottom: 0, left: 0, padding: 10 }}
            >
              <div>
                Player B{" "}
                {this.state.player_b === this.myWalletAddress ? " (You)" : ""}
              </div>
              <div>{this.state.player_b ? this.state.player_b : ""}</div>
            </div>
            <Hand
              style={{ position: "absolute", left: "23vw" }}
              cards={this.state.player_b_hand.map((c) =>
                stateCardToReactCard(c)
              )}
            />
          </Table>
        </center>
      </div>
    );
  }
}

function stateCardToReactCard(c, component = false) {
  let suit = c.suit;
  let value = c.value;

  if (!c.value || !c.suit) {
    if (component) {
      return <Card />;
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
    return <Card face={face} suit={suit} />;
  } else {
    return { face, suit };
  }
}

export default App;
