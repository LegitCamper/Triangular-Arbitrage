# This script is intended to be ran alone. It will index all the pairs and find the chainable ones
from itertools import combinations_with_replacement
from kucoin.client import Market
import json
import os
import time

stable_coins = ["USDT", "TUSD", "BUSD", "USDC", "DAI"]

pairs_catalog = open(f"{os.getcwd()}/Triangular_pairs.catalog", 'w')

client = Market(url="https://api.kucoin.com")


def get_tradable_coin_pairs():
    coin_pairs = []
    for i in client.get_symbol_list():
        if i["enableTrading"]:
            coin_pairs.append(i["symbol"])
    return coin_pairs


def create_catalog():
    json_output = []

    for combo in combinations_with_replacement(get_tradable_coin_pairs(), 3):
        pair1 = combo[0].split("-")
        pair2 = combo[1].split("-")
        pair3 = combo[2].split("-")

        # Needs to pass three requirements:
        # 1) 2 pairs need to have stable coins.
        # 2) the stable coins must only be in the first and third pair
        # 3) I need to be able to chain together the 3 pairs USDT-BTC->BTC-ETH->ETH-USDT
        
        pairs_list = [pair1[0], pair1[1], pair2[0], pair2[1], pair3[0], pair3[1]]

        # Ensures the stable coin only occurs twice and the stable coin is not in the middle pair
        for i in stable_coins:
            if (pairs_list.count(i) == 2 and 
                i != pairs_list[2] and i != pairs_list[3]):
                stable_coin_in_pairs = i

                # Ensures the beginning and end of pairs_list are both stable coins
                if (stable_coin_in_pairs == pairs_list[0] or stable_coin_in_pairs == pairs_list[1] and
                    stable_coin_in_pairs == pairs_list[4] or stable_coin_in_pairs == pairs_list[5]):

                    # Ensure the pairs can chain together
                    if (pairs_list.count(pairs_list[0]) == 2 and
                        pairs_list.count(pairs_list[1]) == 2 and
                        pairs_list.count(pairs_list[2]) == 2 and
                        pairs_list.count(pairs_list[3]) == 2):

                        json_output.append(pairs_list)

    json.dump(json_output, pairs_catalog)


def count_coins_in_catalog():
    with open(f'/home/sawyer/Documents/GitHub/Triangular-Arbitrage/Orderbook/Triangular_pairs.catalog', 'r') as f:
        catalog = json.load(f)
    
    coins_in_catalog = []
    for i in catalog:
        for o in i:
            if o not in coins_in_catalog:
                coins_in_catalog.append(o)

    return len(coins_in_catalog)

if __name__ == "__main__":
    print('This will create the pair catalog - maybe update the stable coins too (takes a couple minutes to run LOL)')
    start_time = time.time()
    create_catalog()
    print(f"Creating the catalog took {(round(time.time() - start_time, 2)) / 60} minutes")

    print("\nbelow is the number of coins in catalog (all coins in kucoin is 1247)")
    print(count_coins_in_catalog())

