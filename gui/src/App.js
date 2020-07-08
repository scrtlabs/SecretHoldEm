import React from "react";
// import { Form, TextArea, Input, Icon } from "semantic-ui-react";
import * as SecretJS from "secretjs";
import * as bip39 from "bip39";
import { Hand, Table, Deck, Card } from "react-casino";

import "semantic-ui-css/semantic.min.css";
import "./App.css";

class App extends React.Component {
  constructor(props) {
    super(props);
    this.state = { community_cards: [], my_hand: [{}, {}] };
  }

  async componentWillMount() {
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
    }, 50);

    setInterval(async () => {
      const data = await secretJsClient.queryContractSmart(
        "secret1xzlgeyuuyqje79ma6vllregprkmgwgavk8y798",
        { get_public_data: {} }
      );

      this.setState({
        community_cards: data.community_cards,
        player_a: data.player_a,
        player_a_bet: data.player_a_bet,
        player_a_hand: data.player_a_hand,
        player_a_wallet: data.player_a_wallet,
        player_b: data.player_b,
        player_b_bet: data.player_b_bet,
        player_b_hand: data.player_b_hand,
        player_b_wallet: data.player_b_wallet,
        stage: data.stage,
        starter: data.starter,
        turn: data.turn,
      });
    }, 3000);
  }

  render() {
    return (
      <div>
        <Table>
          <div style={{ position: "absolute", left: "35%" }}>
            {this.state.community_cards.map((c) => stateCardToReactCard(c))}
          </div>
          <div>
            <Hand
              style={{ position: "absolute", right: "35%" }}
              cards={this.state.my_hand.map((c) => stateCardToReactCard(c))}
            />
          </div>
          <div>
            <Hand
              style={{ position: "absolute", left: "23%" }}
              cards={[{}, {}]}
            />
          </div>
        </Table>
      </div>
    );
  }
}

function stateCardToReactCard({ value, suit }) {
  if (!value || !suit) {
    return {};
  }

  suit = suit[0];
  let face;
  if ((value = +"Two")) {
    face = "2";
  } else if ((value = +"Three")) {
    face = "3";
  } else if ((value = +"Four")) {
    face = "4";
  } else if ((value = +"Five")) {
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

  return { face, suit };
}

export default App;
