import numpy as np
import json
import os


stable_coins = ["USDT", "TUSD", "BUSD", "USDC", "DAI"]

pair_catalog_file = open(f"{os.getcwd()}/Triangular_pairs.catalog", "r")

starting_amount_USD = 5

pair_catalog = json.load(pair_catalog_file)


def round_value(coin_amount):
    if coin_amount == None or coin_amount == 0.0:
        return 0.0
    scientific_to_decimal = np.format_float_positional(coin_amount, trim='-')
    split_value = scientific_to_decimal.split(".")
    if len(split_value) == 1:
        return 0.0
    return (int(coin_amount) + int(split_value[1][:8]) / 100000) # math.floor rounds down, math.ceil round up


def find_tri_arb_path():
    for pairs in pair_catalog:
        #print(pairs) ## DEBUG

        pairs_list = pairs

        pair1 = f"{pairs[0]}-{pairs[1]}"
        pair2 = f"{pairs[2]}-{pairs[3]}"
        pair3 = f"{pairs[4]}-{pairs[5]}"
        
        try:
            with open(f"{os.getcwd()}/Results/{pair1}.kupair") as f:
                pair1_orderbook = json.load(f)
            with open(f"{os.getcwd()}/Results/{pair2}.kupair") as f:
                pair2_orderbook = json.load(f)
            with open(f"{os.getcwd()}/Results/{pair3}.kupair") as f:
                pair3_orderbook = json.load(f)
        except:
            continue

        for i in stable_coins:
            if (pairs_list.count(i) == 2 and 
                i != pairs_list[2] and i != pairs_list[3]):
                stable_coin_in_pairs = i

        
        pair1_asks = pair1_orderbook[pair1]['asks']
        pair1_bids = pair1_orderbook[pair1]['bids']
        pair2_asks = pair2_orderbook[pair2]['asks']
        pair2_bids = pair2_orderbook[pair2]['bids']
        pair3_asks = pair3_orderbook[pair3]['asks']
        pair3_bids = pair3_orderbook[pair3]['bids']

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


        if (len(where_are_stable_coins) == 2 and
            len(where_is_transaction_coin_two) == 2 and
            len(where_is_transaction_coin_three) == 2):

            # MANA-BTC
            # Price in BTC, Amount in MANA 
            # Bid is the price buyers are ready to buy at
            # Ask id the price sellers are ready to sell at
            # The coin on the left is what I am either buying of selling
            # print(int(num) + int(str(num).split(".")[1][:8] / 100000000)) 

            # Calculations
            # Transaction 1
            if where_are_stable_coins[0] == 0:
                if where_are_stable_coins[0] == 'USDT':
                    if float(pair1_bids[0][1]) >= 5:
                        coin_amount = (starting_amount_USD / float(pair1_bids[0][0])) * 0.001
                    else: # if pair1_bids < 5
                        coin_amount = (float(pair1_bids[0][1]) / float(pair1_bids[0][0])) * 0.001
                else:
                    if float(pair1_bids[0][1]) >= 5:
                        coin_amount = ((starting_amount_USD * 0.001) / float(pair1_bids[0][0])) * 0.001 # Accounts for purchases from UTDT to USDC ex.
                    else: # if pair1_bids < 5
                        coin_amount = ((float(pair1_bids[0][1]) * 0.001) / float(pair1_bids[0][0])) * 0.001
            elif where_are_stable_coins[0] == 1:
                if where_are_stable_coins[0] == 'USDT':
                    if float(pair1_asks[0][1]) >= 5:
                        coin_amount = (starting_amount_USD / float(pair1_asks[0][0])) * 0.001
                    else: # if pair1_bids < 5
                        coin_amount = (float(pair1_asks[0][1]) / float(pair1_asks[0][0])) * 0.001
                else:
                    if float(pair1_asks[0][1]) >= 5:
                        coin_amount = ((starting_amount_USD * 0.001) / float(pair1_asks[0][0])) * 0.001 # Accounts for purchases from UTDT to USDC ex.
                    else: # if pair1_bids < 5
                        coin_amount = ((float(pair1_asks[0][1]) * 0.001) / float(pair1_asks[0][0])) * 0.001
            coin_amount = round_value(coin_amount)
            if coin_amount == 0:
                continue
            
            # Transaction 2
            if where_is_transaction_coin_two[1] == 2:
               if float(pair2_bids[0][1]) >= 5:
                    coin_amount = (coin_amount / float(pair2_bids[0][0])) * 0.001
               else: # if pair1_bids < 5
                    coin_amount = (float(pair2_bids[0][1]) / float(pair2_bids[0][0])) * 0.001
            elif where_is_transaction_coin_two[1] == 3:
                if float(pair2_asks[0][1]) >= 5:
                    coin_amount = (coin_amount / float(pair2_asks[0][0])) * 0.001
                else: # if pair1_bids < 5
                    coin_amount = (float(pair2_asks[0][1]) / float(pair2_asks[0][0])) * 0.001
            coin_amount = round_value(coin_amount)
            if coin_amount == 0:
                continue

            # Transaction 3
            if where_is_transaction_coin_three[1] == 4:
               if float(pair3_bids[0][1]) >= 5:
                    coin_amount = (coin_amount / float(pair3_bids[0][0])) * 0.001
               else: # if pair1_bids < 5
                    coin_amount = (float(pair3_bids[0][1]) / float(pair3_bids[0][0])) * 0.001
            elif where_is_transaction_coin_three[1] == 5:
                if float(pair3_asks[0][1]) >= 5:
                    coin_amount = (coin_amount / float(pair3_asks[0][0])) * 0.001
                else: # if pair1_bids < 5
                    coin_amount = (float(pair3_asks[0][1]) / float(pair3_asks[0][0])) * 0.001
            coin_amount = round_value(coin_amount)
            if coin_amount == 0:
                continue

            # Transaction 4 - If need to exchange back to USDT
            if where_are_stable_coins[0] == 'USDT':
                coin_amount = coin_amount * 0.001
            coin_amount = round_value(coin_amount)

            

            if starting_amount_USD < coin_amount:
                print(f"\nI now have {coin_amount}, which means a net of ${coin_amount-starting_amount_USD}")
                print("I made money")

    # Logic to determine if a path is availibe


if __name__ == "__main__":
    import time
    while True:
        start_time = time.time()
        find_tri_arb_path()
        print(f'Algo took {str(time.time() - start_time)[:8]} seconds')
