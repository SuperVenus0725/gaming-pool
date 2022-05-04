import logging

from core.constants import GAMING_CONTRACT_PATH, GAMING_INIT, FURY_CONTRACT_ADDRESS, LIQUIDITY_PROVIDER
from core.engine import Engine
from terra_sdk.client.lcd import Wallet

logger = logging.getLogger(__name__)


class GamingTestEngine(Engine):
    def __init__(self, debug, admin_wallet_memonic=None, admin_shift=None):
        super().__init__(debug, admin_wallet_memonic, admin_shift)
        logger.info("Setting Up Gaming Contract")
        self.contract_id = self.upload_wasm(self.admin_wallet, GAMING_CONTRACT_PATH)
        self.game_id = "Game001"
        GAMING_INIT['admin_address'] = self.admin_wallet.key.acc_address
        self.gaming_contract_address = self.instantiate(
            self.admin_wallet,
            str(self.contract_id),
            GAMING_INIT
        )
        self.pool_fee = "10000"
        logger.info(f"Gaming Contract Address {self.gaming_contract_address}")

    def setup(self) -> bool:
        logger.info("Setting Pool Headers")
        response = self.sign_and_execute_contract(self.admin_wallet, self.gaming_contract_address, [{
            "set_pool_type_params": {
                'pool_type': "H2H",
                'pool_fee': self.pool_fee,
                'min_teams_for_pool': 1,
                'max_teams_for_pool': 10000,
                'max_teams_for_gamer': 12,
                'wallet_percentages': [
                    {
                        "wallet_address": "terra1uyuy363rzun7e6txdjdemqj9zua9j7wxl2lp0m",
                        "wallet_name": "rake_1",
                        "percentage": 100,
                    }
                ]
            }
        }])
        logger.info(f"Setting Headers Hash {response.txhash}")
        self.sleep()
        logger.info("Creating Pool")
        response = self.sign_and_execute_contract(self.admin_wallet, self.gaming_contract_address, [{
            "create_pool": {
                "pool_type": "H2H"
            }
        }])
        logger.info(f"Response Of Create Pool {response.txhash}")
        return True

    def perform_bidsubmit(self, wallet: Wallet, pool_type="H2H", pool_id="1", team_id="Team001"):
        logger.info("Loading Balance")
        logger.info(f"Current Gamer {wallet.key.acc_address}")
        # self.load_fury(wallet.key.acc_address, "5000000000")
        logger.info("Getting Funds To Send In Fury")
        funds_to_send = self.get_fury_equivalent_to_ust(self.pool_fee)
        logger.info(f"Sending {funds_to_send} FURY")
        self.load_fury(wallet.key.acc_address, str(funds_to_send))
        logger.info(f"Increasing Allowance On the Gaming for {funds_to_send}")
        response = self.execute(wallet, FURY_CONTRACT_ADDRESS, {
            "increase_allowance": {
                "spender": self.gaming_contract_address,
                "amount": str(funds_to_send)
            }
        })
        logger.info(f"Response Of Increase Allowance {response.txhash}")
        spread = self.simulate_swap_ust(funds_to_send)
        logger.info(f"Max Spread: {spread}")
        logger.info("Submitting Bid")
        response = self.execute(wallet, self.gaming_contract_address, {
            'game_pool_bid_submit_command': {
                'gamer': wallet.key.acc_address,
                'pool_type': pool_type,
                'pool_id': pool_id,
                'team_id': team_id,
                'amount': str(funds_to_send),
                "max_spread": str(spread)
            }
        }, {"uusd": "1100000"})
        logger.info(f"Response Of Bid Submit {response.txhash}")

    def lock_game_and_swap_balance(self, balance_to_swap: int):
        logger.info("Executing Lock Game")
        response = self.sign_and_execute_contract(self.admin_wallet, self.gaming_contract_address, [{
            "lock_game": {}
        }])
        logger.info(f"Response Of Lockgame {response.txhash}")
        logger.info(f"Performing Swap for the balance {balance_to_swap} $UST to $FURY")
        spread = self.simulate_swap_ust(balance_to_swap)
        logger.info(f"Max Spread: {spread}")
        response = self.sign_and_execute_contract(self.admin_wallet, self.gaming_contract_address, [{
            "swap": {
                "amount": str(int(balance_to_swap - 200000)),
                "pool_id": "1",
                "max_spread": str(spread)
            }
        }])
        logger.info(f"Swap Response {response.txhash}")

    def reward_distribution_for_users(self, users: [Wallet]):
        """
        This method accepts a list of user wallets ana auto distributes the batches according the list size
        :param users:
        :return:
        """
        logger.info(f"Executing Reward Distribution for {len(users)} Users")
        if len(users) > 10:
            batches = self.divide_to_batches(users, 2)
        else:
            batches = [users]
        ust_for_rake = self.query_contract(self.gaming_contract_address, {
            "swap_info": {
                "pool_id": "1"
            }
        })['ust_for_rake']
        logger.info(f"$UST Left for Rake {ust_for_rake}")
        msgs = []
        logger.info("Batching and Executing Reward Distribute")
        for batch in batches:
            is_last_batch = batch == list(batches)[-1]
            winners = []
            for user in batch:
                winners.append({
                    "gamer_address": user.key.acc_address,
                    "game_id": self.game_id,
                    "team_id": "Team001",
                    "reward_amount": "5000",
                    "refund_amount": "0",
                    "team_rank": 1,
                    "team_points": 150
                })
            msgs.append(
                {
                    "game_pool_reward_distribute": {
                        "is_final_batch": is_last_batch,
                        "game_id": self.game_id,
                        "pool_id": "1",
                        "ust_for_rake": f"{ust_for_rake}",
                        "game_winners": winners,
                    }
                }
            )
        response = self.sign_and_execute_contract(self.admin_wallet, self.gaming_contract_address, msgs)
        logger.info(f"Response Hash For reward Distribute {response.txhash}")

    def claim_reward(self, wallet: Wallet):
        logger.info(f"Claiming Reward For {wallet.key.acc_address}")
        expected_reward = self.query_contract(self.gaming_contract_address, {
            "query_reward": {"gamer": wallet.key.acc_address}
        })
        logger.info(f"Expected Reward Amount : {expected_reward}")
        response = self.execute(wallet, self.gaming_contract_address, {
            "claim_reward": {"gamer": wallet.key.acc_address}
        }, {"uusd": "1000000"})
        logger.info(f"Claim Reward Response Hash {response.txhash}")

    def run_test_1(self, number_of_users):
        logger.info(f"Running Test Setup with {number_of_users} Users Placing Bid")
        self.setup()
        wallets_for_test = self.generate_wallets(number_of_users)
        self.fund_wallets(wallets_for_test)
        for wallet in wallets_for_test:
            self.perform_bidsubmit(wallet)
        if number_of_users > 5:
            logger.info("Seeding the liquidity provider")
            # More users require us to seed the LP so there is enough liquidity to perform swap
            self.fund_wallet(LIQUIDITY_PROVIDER, f"{10000000000 * number_of_users}", 2000000000 * number_of_users)
        self.lock_game_and_swap_balance(int(self.pool_fee) * number_of_users)
        self.reward_distribution_for_users(wallets_for_test)
        for wallet in wallets_for_test:
            self.claim_reward(wallet)

    #
    # def run_test_1_optimized(self, number_of_users):
    #     logger.info(f"Running Test Setup with {number_of_users} Users Placing Bid")
    #     self.setup()
    #     wallets_for_test = self.generate_wallets(number_of_users)
    #     self.fund_wallets(wallets_for_test)
    #     # We break them into batches of 500 and then each thread will run manage the bidsubmit for that queue
    #     batches = self.divide_to_batches(wallets_for_test, 500)
    #     threads = []
    #     for batch in batches:
    #         threads.append(
    #
    #         )
