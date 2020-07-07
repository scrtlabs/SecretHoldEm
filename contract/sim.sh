#!/bin/bash

CODE_ID=$(
    secretcli tx compute store contract.wasm.gz --from mykey -y --gas 10000000 -b block |
        jq -r '.logs[].events[].attributes[] | select(.key == "code_id") | .value'
)

CONTRACT=$(
    secretcli tx compute instantiate "$CODE_ID" "{}" --from mykey --label test -y -b block |
        jq -r '.logs[].events[].attributes[] | select(.key == "contract_address") | .value'
)

secretcli tx compute execute "$CONTRACT" '{"join":{"secret":123}}' --from 1 -b block -y |
    jq .txhash |
    xargs secretcli q compute tx |
    jq -r .output_data |
    base64 -d
echo 

secretcli tx compute execute "$CONTRACT" '{"join":{"secret":234}}' --from 2 -b block -y |
    jq .txhash |
    xargs secretcli q compute tx |
    jq -r .output_data |
    base64 -d
echo

echo Player A Hand:
secretcli q compute contract-state smart "$CONTRACT" '{"get_my_hand":{"secret":123}}'

echo Player B Hand:
secretcli q compute contract-state smart "$CONTRACT" '{"get_my_hand":{"secret":234}}'

echo Table:
secretcli q compute contract-state smart "$CONTRACT" '{"get_public_data":{}}' | jq .