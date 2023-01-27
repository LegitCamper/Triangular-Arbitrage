# This script is intended to be ran alone. It will index all the pairs and find the chainable ones
# It takes quite a while to run so you can compile and run it with:
# nuitka3-run --quiet --remove-output --output-dir=bin create_valid_pairs_catalog.py
from itertools import combinations_with_replacement
from kucoin.client import Market
import json
import os
import time

stable_coins = ["USDT", "TUSD", "BUSD", "USDC", "DAI"]

pairs_catalog_path = f"{os.getcwd()}/Triangular_pairs.catalog"
if os.path.exists(pairs_catalog_path):
    os.remove(pairs_catalog_path)
pairs_catalog = open(pairs_catalog_path, 'w')

client = Market(url="https://api.kucoin.com")


def get_tradable_coin_pairs():
    coin_pairs = []
    for i in client.get_symbol_list():
        if i["enableTrading"]:
            coin_pairs.append(i["symbol"])
    return coin_pairs

# Needs to pass three requirements:
# 1) 2 pairs need to have stable coins.
# 2) the stable coins must only be in the first and third pair
# 3) I need to be able to chain together the 3 pairs USDT-BTC->BTC-ETH->ETH-USDT
def create_catalog():
    json_output = []
    coin_pairs = get_tradable_coin_pairs()

    for pair1 in coin_pairs:
        for pair2 in coin_pairs:
            for pair3 in coin_pairs:

                pairs_list = [pair1[0], pair1[1], pair2[0], pair2[1], pair3[0], pair3[1]]

                # Ensure the pairs can chain together
                if (pairs_list.count(pairs_list[0]) == 2 and
                    pairs_list.count(pairs_list[1]) == 2 and
                    pairs_list.count(pairs_list[2]) == 2 and
                    pairs_list.count(pairs_list[3]) == 2):

                        # First and last pair have a stable coin
                        for i in stable_coins:
                            if i in pair1 and i in pair3:
                                i_ = i
                        
                        try:
                            # Ensures the beginning and end of pairs_list are both stable coins
                            if (i_ == pairs_list[0] or i_ == pairs_list[1] and
                                i_ == pairs_list[4] or i_ == pairs_list[5]):

                                    json_output.append(pairs_list)
                        except:
                            continue

    json.dump(json_output, pairs_catalog)

    
def count_coins_in_catalog():
    catalog = json.load(pairs_catalog)
    
    coins_in_catalog = []
    for i in catalog:
        for o in i:
            if o not in coins_in_catalog:
                coins_in_catalog.append(o)

    return len(coins_in_catalog)

if __name__ == "__main__":
    print('This will create the pair catalog (takes a couple minutes to run)')
    start_time = time.time()
    create_catalog()
    print(f"Creating the catalog took {(round(time.time() - start_time, 2)) / 60} minutes")

    #print("\nbelow is the number of unique coins in catalog (all coins in kucoin is 1247)")
    #print(count_coins_in_catalog())

