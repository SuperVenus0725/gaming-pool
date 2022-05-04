import logging

from terra_sdk.client.lcd import Wallet

from load_testing.core.constants import CLUB_STAKING_CONTRACT_PATH, CLUB_STAKING_INIT, FURY_CONTRACT_ADDRESS
from load_testing.core.engine import Engine

"""
Notes
Currently the testing on local_terra gets limited due the shared accounts and their execution limits,
in order to avoid this its always a good practice to create tests that work within segregated accounts.
Use 10 admins and all the other roles should come with 
"""
logger = logging.getLogger(__name__)


class StakingTestEngine(Engine):
    def __init__(self, debug, admin_wallet_memonic=None, admin_shift=None):
        super().__init__(debug, admin_wallet_memonic, admin_shift)
        logger.info("Staking Test Instantiated, Setting new Club owners")
        self.club_owners = self.generate_wallets(2)
        self.auto_stake = True
        self.amount_to_stake_per_club = "100000"
        [self.fund_wallet(owner) for owner in self.club_owners]
        self.contract_id = self.upload_wasm(self.admin_wallet, CLUB_STAKING_CONTRACT_PATH)
        CLUB_STAKING_INIT['admin_address'] = self.admin_wallet.key.acc_address
        CLUB_STAKING_INIT['platform_fees_collector_wallet'] = self.admin_wallet.key.acc_address
        CLUB_STAKING_INIT['club_fee_collector_wallet'] = self.admin_wallet.key.acc_address
        self.club_staking_address = self.instantiate(self.admin_wallet, str(self.contract_id), CLUB_STAKING_INIT)

    @staticmethod
    def get_club_name(owner: Wallet):
        return f"Club_{owner.key.acc_address}"

    def buy_club(self, wallet: Wallet):
        self.increase_allowance(wallet, self.club_staking_address, "100000")
        buy_a_club_request = {
            "buy_a_club": {
                'buyer': wallet.key.acc_address,
                'club_name': self.get_club_name(wallet),
                'auto_stake': self.auto_stake
            }
        }
        logger.info("Getting Platform Fees for the Purchase")
        platform_fees = self.query_contract(self.club_staking_address, {
            "query_platform_fees": {
                "msg": self.base64_encode_dict(buy_a_club_request)
            }
        })
        logger.info(f"Platform Fee For The Purchase {platform_fees}")
        logger.info(f"Buying Club {self.get_club_name(wallet)} With {wallet.key.acc_address}")
        response = self.execute(wallet, self.club_staking_address, buy_a_club_request,
                                {"uusd": str(platform_fees)})
        logger.info(f"Buy a club response: {response.txhash}")

    def setup_clubs(self):
        for owner in self.club_owners:
            self.buy_club(owner)

    def stake_to_club(self, wallet: Wallet, club_name: str):
        logger.info(f"Initiating Staaking for {wallet.key.acc_address} On Club {club_name}")
        self.increase_allowance(wallet, self.club_staking_address, self.amount_to_stake_per_club)
        logger.info("Getting Platform Fees For Staking On The Club")
        stake_on_a_club_request = {
            'stake_on_a_club': {
                'staker': wallet.key.acc_address,
                'club_name': club_name,
                'amount': self.amount_to_stake_per_club,
                'auto_stake': self.auto_stake,
            }
        }
        platform_fees = self.query_contract(self.club_staking_address, {
            "query_platform_fees": {
                "msg": self.base64_encode_dict(stake_on_a_club_request)
            }
        })
        logger.info(f"Response Of Platform Fees {platform_fees}")
        logger.info("Executing Stake On a Club")
        response = self.execute(
            wallet,
            self.club_staking_address,
            stake_on_a_club_request,
            {"uusd": platform_fees}
        )
        logger.info(f"Staking On a Club TX Hash {response.txhash}")

    def query_stakes(self, club_name, users):
        logger.info(f"Initiate Query For Club Stakes for {len(users)} Users on Club {club_name}")
        batches = self.divide_to_batches(users, 2)
        for batch in batches:
            b = [b.key.acc_address for b in batch]
            response = self.query_contract(self.club_staking_address, {
                'club_staking_details': {
                    'club_name': club_name,
                    'user_list': b
                }
            })
            logger.info(f"Response Of Stakes: \n{response}")

    def increase_reward(self, amount):
        logger.info(f"Executing Increase Reward Amount to {amount}")
        irs_request = {
            "increase_reward_amount": {
                "reward_from": self.admin_wallet.key.acc_address
            }
        }
        encoded = self.base64_encode_dict(irs_request)
        via_msg = {
            "send": {
                "contract": self.club_staking_address,
                "amount": str(amount),
                "msg": encoded
            }
        }
        response = self.execute(self.admin_wallet, FURY_CONTRACT_ADDRESS, via_msg)
        logger.info(f"Increase Reward Amount Response {response.txhash}")

    def distribute_reward_per_batch(self, club_name, users):
        logger.info("Executing Reward Distribute in Batches")
        batches = list(self.divide_to_batches(users, 500))
        logger.info(f"Batch-Sized to {len(batches)} Batches")
        for batch in batches:
            is_first = batch == batches[0]
            is_last = batch == batches[-1]
            b = [b.key.acc_address for b in batch]
            response = self.execute(self.admin_wallet, self.club_staking_address, {
                "calculate_and_distribute_rewards": {
                    "staker_list": b,
                    "club_name": club_name,
                    "is_first_batch": is_first,
                    "is_final_batch": is_last
                }
            })
            logger.info(f"Calculate and distribute reward response hash {response.txhash}")

    def run_test_1(self, number_of_users):
        self.setup_clubs()
        logger.info(f"Loading {number_of_users} Users for Test")
        wallets_for_test = self.generate_wallets(number_of_users)
        self.fund_wallets(wallets_for_test)
        for wallet in wallets_for_test:
            for owner in self.club_owners:
                self.stake_to_club(wallet, self.get_club_name(owner))
        for owner in self.club_owners:
            self.query_stakes(self.get_club_name(owner), wallets_for_test)
        self.increase_reward(str(int((number_of_users * int(self.amount_to_stake_per_club)) / 10)).split('.')[0])
        for owner in self.club_owners:
            self.distribute_reward_per_batch(self.get_club_name(owner), wallets_for_test)
        for owner in self.club_owners:
            self.query_stakes(self.get_club_name(owner), wallets_for_test)
