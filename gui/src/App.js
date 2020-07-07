import React from "react";
// import { Form, TextArea, Input, Icon } from "semantic-ui-react";
import * as SecretJS from "secretjs";

import "semantic-ui-css/semantic.min.css";
import "./App.css";

let tx_encryption_seed = localStorage.getItem("tx_encryption_seed");
if (!tx_encryption_seed) {
  tx_encryption_seed = SecretJS.EnigmaUtils.GenerateNewSeed();
  localStorage.setItem("tx_encryption_seed", tx_encryption_seed);
} else {
  tx_encryption_seed = Uint8Array.from(JSON.parse(`[${tx_encryption_seed}]`));
}

let secretJsClient;
SecretJS.Secp256k1Pen.fromMnemonic(
  "bracket master kitten deposit favorite exhibit rare news before depend address gap boil salt tennis brown room return clap squirrel project scissors aim whale"
).then((signingPen) => {
  const myWalletAddress = SecretJS.pubkeyToAddress(
    SecretJS.encodeSecp256k1Pubkey(signingPen.pubkey),
    "secret"
  );
  secretJsClient = new SecretJS.SigningCosmWasmClient(
    "http://bootstrap.int.testnet.enigma.co:1337",
    myWalletAddress,
    (signBytes) => signingPen.sign(signBytes),
    tx_encryption_seed,
    {
      init: {
        amount: [{ amount: "12500", denom: "uscrt" }],
        gas: "500000",
      },
      exec: {
        amount: [{ amount: "5000", denom: "uscrt" }],
        gas: "200000",
      },
    }
  );
});

class App extends React.Component {
  constructor(props) {
    super(props);
    this.state = { data: "" };

    setInterval(async () => {
      if (!secretJsClient) {
        return;
      }

      const data = await secretJsClient.queryContractSmart(
        "secret1ag9uu96j27tvufjqg6yleq49gg6lahf808c8l6",
        { get_public_data: {} }
      );

      this.setState({ data: JSON.stringify(data) });
    }, 1000);
  }

  render() {
    return <div>{this.state.data}</div>;
  }
}

export default App;
