import json

with open('json/localterra.json', 'r') as f:
    deployment_details = json.load(f)

# CONTRACT PATHS
GAMING_CONTRACT_PATH = "../artifacts/gaming_pool.wasm"
CLUB_STAKING_CONTRACT_PATH = "../artifacts/club_staking.wasm"
# LOAD ADDRESS FROM JSON
FURY_CONTRACT_ADDRESS = deployment_details.get("furyContractAddress")
PROXY_CONTRACT_ADDRESS = deployment_details.get("proxyContractAddress")
LIQUIDITY_PROVIDER = deployment_details.get("authLiquidityProvider")
# CONTRACT INIT
GAMING_INIT = {
    "minting_contract_address": FURY_CONTRACT_ADDRESS,
    "admin_address": "",
    "platform_fee": "100",
    "transaction_fee": "30",
    "game_id": "Game001",
    "platform_fees_collector_wallet": PROXY_CONTRACT_ADDRESS,
    "astro_proxy_address": PROXY_CONTRACT_ADDRESS,
}
CLUB_STAKING_INIT = {
    "admin_address": "",
    "minting_contract_address": FURY_CONTRACT_ADDRESS,
    "astro_proxy_address": PROXY_CONTRACT_ADDRESS,
    "platform_fees_collector_wallet": "",
    "club_fee_collector_wallet": "",
    "club_reward_next_timestamp": "1640447808000000000",
    "reward_periodicity": 300,
    "club_price": "100000",
    "bonding_duration": 120,
    "platform_fees": "100",
    "transaction_fees": "30",
    "control_fees": "50",
    "max_bonding_limit_per_user": 100,
    "owner_release_locking_duration": 0
}

MINTING_WALLET_MEMONIC = "awesome festival volume rifle diagram suffer rhythm knock unlock reveal marine transfer lumber faint walnut love hover beach amazing robust oppose moon west will"
