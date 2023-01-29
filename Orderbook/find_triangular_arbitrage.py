import numpy as np
import json
import os
import shutil


stable_coins = ["USDT", "TUSD", "BUSD", "USDC", "DAI"]

pair_catalog_file = open(f"{os.getcwd()}/Triangular_pairs.catalog", "r")

starting_amount_USD = 100

pair_catalog = json.load(pair_catalog_file)


def round_value(coin_amount):
    if coin_amount == None or coin_amount == 0.0:
        return 0.0
    scientific_to_decimal = np.format_float_positional(coin_amount, trim='-')
    split_value = scientific_to_decimal.split(".")
    if len(split_value) == 1:
        return 0.0
    return float(f'0.{split_value[1][:8]}') # math.floor rounds down, math.ceil round up


def Read_File(path):
    shutil.copy(path, f"{os.getcwd()}/TempRead.kupair")

    with open(f"{os.getcwd()}/TempRead.kupair", "r") as f:
        return json.load(f)


def find_tri_arb_path():
    for pairs in pair_catalog:
        pairs_list = pairs
               
        pair1 = f"{pairs[0]}-{pairs[1]}"
        pair2 = f"{pairs[2]}-{pairs[3]}"
        pair3 = f"{pairs[4]}-{pairs[5]}"

        try:
            pair1_orderbook = Read_File(f"{os.getcwd()}/Results/{pair1}.kupair")
            pair2_orderbook = Read_File(f"{os.getcwd()}/Results/{pair2}.kupair")
            pair3_orderbook = Read_File(f"{os.getcwd()}/Results/{pair3}.kupair")
        except:
            continue

        # Finds what the stable coin is 
        if pairs_list[0] == pairs_list[4]:
            stable_coin_in_pairs = pairs_list[0]
        if pairs_list[0] == pairs_list[5]:
            stable_coin_in_pairs = pairs_list[0]
        if pairs_list[1] == pairs_list[4]:
            stable_coin_in_pairs = pairs_list[1]
        if pairs_list[1] == pairs_list[5]:
            stable_coin_in_pairs = pairs_list[1]
             
        pair1_asks = pair1_orderbook[pair1]['asks']
        pair1_bids = pair1_orderbook[pair1]['bids']
        pair2_asks = pair2_orderbook[pair2]['asks']
        pair2_bids = pair2_orderbook[pair2]['bids']
        pair3_asks = pair3_orderbook[pair3]['asks']
        pair3_bids = pair3_orderbook[pair3]['bids']

        try: # prevents index errors
            # Transaction 1 Check
            where_are_stable_coins = [] # [0, 4]
            for index, item in enumerate(pairs_list):
                if item == stable_coin_in_pairs:
                    where_are_stable_coins.append(index)

            # Transaction 2 Check
            where_is_transaction_coin_two = [] # [1, 2]
            if where_are_stable_coins[0] == 0:
                where_is_transaction_coin_two.append(1)
                where_is_transaction_coin_two.append(pairs_list.index(pairs_list[0], 1))
            elif where_are_stable_coins[0] == 1:
                where_is_transaction_coin_two.append(0)
                where_is_transaction_coin_two.append(pairs_list.index(pairs_list[0], 1))

            # Transaction 3 Check
            where_is_transaction_coin_three = []
            if where_is_transaction_coin_two[1] == 2:
                where_is_transaction_coin_three.append(3)
            elif where_is_transaction_coin_two[1] == 3:
                where_is_transaction_coin_three.append(2)
            if where_are_stable_coins[1] == 4:
                where_is_transaction_coin_three.append(5)
            elif where_are_stable_coins[1] == 5:
                where_is_transaction_coin_three.append(4)
        except:
            continue

        #print("\n", pairs_list) ### Debug
        #print(where_are_stable_coins) ### Debug
        #print(where_is_transaction_coin_two) ### Debug
        #print(where_is_transaction_coin_three) ### Debug

        if (len(where_are_stable_coins) == 2 and
            len(where_is_transaction_coin_two) == 2 and
            len(where_is_transaction_coin_three) == 2):

            # MANA-BTC
            # Price in BTC, Amount in MANA 
            # Bid is the price buyers are ready to buy at
            # Ask id the price sellers are ready to sell at
            # The coin on the left is what I am either buying of selling

            # Calculations
            # Transaction 1
            if where_are_stable_coins[0] == 0:
                if where_are_stable_coins[0] == 'USDT':
                    coin_amount = (starting_amount_USD / float(pair1_bids[4][0])) * 0.001
                else:
                    coin_amount = ((starting_amount_USD * 0.001) / float(pair1_bids[4][0])) * 0.001 # Accounts for purchases from UTDT to USDC ex.
            elif where_are_stable_coins[0] == 1:
                if where_are_stable_coins[0] == 'USDT':
                    coin_amount = (starting_amount_USD / float(pair1_asks[0][0])) * 0.001
                else:
                    coin_amount = ((starting_amount_USD * 0.001) / float(pair1_asks[0][0])) * 0.001 # Accounts for purchases from UTDT to USDC ex.
            coin_amount = round_value(coin_amount)
            if coin_amount == 0:
                continue
            
            # Transaction 2
            if where_is_transaction_coin_two[1] == 2:
                coin_amount = (coin_amount / float(pair2_bids[4][0])) * 0.001
            elif where_is_transaction_coin_two[1] == 3:
                coin_amount = (coin_amount / float(pair2_asks[0][0])) * 0.001
            coin_amount = round_value(coin_amount)
            if coin_amount == 0:
                continue

            # Transaction 3
            if where_is_transaction_coin_three[1] == 4:
                coin_amount = (coin_amount / float(pair3_bids[4][0])) * 0.001
            elif where_is_transaction_coin_three[1] == 5:
                coin_amount = (coin_amount / float(pair3_asks[0][0])) * 0.001
            coin_amount = round_value(coin_amount)
            if coin_amount == 0:
                continue

            # Transaction 4 - If need to exchange back to USDT
            if where_are_stable_coins[0] == 'USDT':
                coin_amount = coin_amount * 0.001
                coin_amount = round_value(coin_amount)

            

            if starting_amount_USD < coin_amount:
                print(f"\n For pair: {pairs}\nI now have {coin_amount}\nWhich means a net of ${coin_amount-starting_amount_USD}")
                print("I made money")


if __name__ == "__main__":
    while True:
        find_tri_arb_path()