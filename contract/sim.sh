#!/bin/bash

set -ve

CODE_ID=$(
    secretcli tx compute store contract.wasm.gz --from 1 -y --gas 10000000 -b block |
        jq -r '.logs[].events[].attributes[] | select(.key == "code_id") | .value'
)

CONTRACT=$(
    secretcli tx compute instantiate "$CODE_ID" "{}" --from 1 --label test -y -b block |
        jq -r '.logs[].events[].attributes[] | select(.key == "contract_address") | .value'
)

secretcli tx compute execute "$CONTRACT" '{"join":{"secret":123}}' --from 1 -b block -y |
    jq .txhash |
    xargs secretcli q compute tx |
    jq -r .output_data |
    base64 -d

secretcli tx compute execute "$CONTRACT" '{"join":{"secret":234}}' --from 3 -b block -y |
    jq .txhash |
    xargs secretcli q compute tx |
    jq -r .output_data |
    base64 -d

# Player A Hand:
secretcli q compute contract-state smart "$CONTRACT" '{"get_my_hand":{"secret":123}}'

# Player B Hand:
secretcli q compute contract-state smart "$CONTRACT" '{"get_my_hand":{"secret":234}}'

# Table:
secretcli q compute contract-state smart "$CONTRACT" '{"get_public_data":{}}' | jq .

# A checks
secretcli tx compute execute "$CONTRACT" '{"check":{}}' --from 1 -b block -y |
    jq .txhash |
    xargs secretcli q compute tx |
    jq .

# B checks
secretcli tx compute execute "$CONTRACT" '{"check":{}}' --from 3 -b block -y |
    jq .txhash |
    xargs secretcli q compute tx |
    jq .

# Table:
secretcli q compute contract-state smart "$CONTRACT" '{"get_public_data":{}}' | jq .

# A checks
secretcli tx compute execute "$CONTRACT" '{"check":{}}' --from 1 -b block -y |
    jq .txhash |
    xargs secretcli q compute tx |
    jq .

# B checks
secretcli tx compute execute "$CONTRACT" '{"check":{}}' --from 3 -b block -y |
    jq .txhash |
    xargs secretcli q compute tx |
    jq .

# Table:
secretcli q compute contract-state smart "$CONTRACT" '{"get_public_data":{}}' | jq .

# A checks
secretcli tx compute execute "$CONTRACT" '{"check":{}}' --from 1 -b block -y |
    jq .txhash |
    xargs secretcli q compute tx |
    jq .

# B checks
secretcli tx compute execute "$CONTRACT" '{"check":{}}' --from 3 -b block -y |
    jq .txhash |
    xargs secretcli q compute tx |
    jq .

# Table:
secretcli q compute contract-state smart "$CONTRACT" '{"get_public_data":{}}' | jq .

echo $CONTRACT
exit

# A checks
secretcli tx compute execute "$CONTRACT" '{"check":{}}' --from 1 -b block -y |
    jq .txhash |
    xargs secretcli q compute tx |
    jq .

# B checks
secretcli tx compute execute "$CONTRACT" '{"check":{}}' --from 3 -b block -y |
    jq .txhash |
    xargs secretcli q compute tx |
    jq .

# Table:
secretcli q compute contract-state smart "$CONTRACT" '{"get_public_data":{}}' | jq .