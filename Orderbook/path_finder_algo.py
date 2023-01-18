from itertools import combinations
import numpy as np
import json
import math
import os


stable_coins = ["USDT", "TUSD", "BUSD", "USDC", "DAI"]


def find_tri_arb_path():
    pairs_catalog = open(f"{os.getcwd()}/triangular_pairs.catalog", 'r')
    for pairs_catalog.readlines():
        ############## FIX
        with open(f"{os.getcwd()}/Results/{combo[0]}", 'r') as f:
            pair1_orderbook = json.loads(f.read())
        with open(f"{os.getcwd()}/Results/{combo[1]}", 'r') as f:
            pair2_orderbook = json.loads(f.read())
        with open(f"{os.getcwd()}/Results/{combo[2]}", 'r') as f:
            pair3_orderbook = json.loads(f.read())

        pair1 = list(pair1_orderbook.keys())[0]
        pair2 = list(pair2_orderbook.keys())[0]
        pair3 = list(pair3_orderbook.keys())[0]
        #########################


        if stable_coins_check and right_order_check and pairs_chain_check:
            pair1_asks = pair1_orderbook[pair1]['asks']
            pair1_bids = pair1_orderbook[pair1]['bids']
            pair2_asks = pair2_orderbook[pair2]['asks']
            pair2_bids = pair2_orderbook[pair2]['bids']
            pair3_asks = pair3_orderbook[pair3]['asks']
            pair3_bids = pair3_orderbook[pair3]['bids']

            # MANA-BTC
            # Price in BTC, Amount in MANA 
            # Bid is the price buyers are ready to buy at
            # Ask id the price sellers are ready to sell at
            # The coin on the left is what I am either buying of selling
            # print(int(num) + int(str(num).split(".")[1][:8] / 100000000)) 

            # Transaction 1
            where_are_stable_coins = [] # [0, 4]
            for index, item in enumerate(pairs_list):
                if item == stable_coin_in_pairs:
                    where_are_stable_coins.append(index)

            # Transaction 2
            where_is_transaction_coin_two = [] # [1, 2]
            if where_are_stable_coins[0] == 0:
                where_is_transaction_coin_two.append(1)
                where_is_transaction_coin_two.append(pairs_list.index(pairs_list[1], 1))
            if where_are_stable_coins[0] == 1:
                where_is_transaction_coin_two.append(0)
                where_is_transaction_coin_two.append(pairs_list.index(pairs_list[0], 1))

            # Transaction 3
            print()
            print(combo)
            print(where_are_stable_coins)
            print(where_is_transaction_coin_two)
            where_is_transaction_coin_three = [] # [3, 5]
            if where_is_transaction_coin_two[1] == 2:
                where_is_transaction_coin_three.append(3)
            if where_is_transaction_coin_two[1] == 3:
                where_is_transaction_coin_three.append(2)
            if where_are_stable_coins[1] == 4:
                where_is_transaction_coin_three.append(5)
            if where_are_stable_coins[1] == 5:
                where_is_transaction_coin_three.append(4)
            
            # Calculates if price is feasible
            # Transaction 1
            coin_amount = 0
            starting_amount_USD = 5 # needs to be the smallest (ask or bid) in all the pair files 
            smallest_amount = 0 # This is the smallest order in the chain and will be the amount I trade
            if where_are_stable_coins[0] == 0:
                if where_are_stable_coins[0] == "USDT":
                    if float(pair1_bids[0][1]) >= 5:
                        coin_amount = (float(pair1_bids[0][0]) / starting_amount_USD) * 0.001
                    else: # if pair1_bids < 5
                        coin_amount = (float(pair1_bids[0][0]) / float(pair1_bids[0][1])) * 0.001
                else:
                    if float(pair1_bids[0][1]) >= 5:
                        coin_amount = (float(pair1_bids[0][0]) / starting_amount_USD * 0.001) * 0.001 # Accounts for purchases from UTDT to USDC ex.
                    else: # if pair1_bids < 5
                        coin_amount = (float(pair1_bids[0][0]) / float(pair1_bids[0][1])) * 0.001

            if where_are_stable_coins[0] == 1:
                if where_are_stable_coins[0] == 'USDT':
                    if float(pair1_asks[0][1]) >= 5:
                        coin_amount = (float(pair1_asks[0][0]) / starting_amount_USD) * 0.001
                    else: # if pair1_bids < 5
                        coin_amount = (float(pair1_asks[0][0]) / float(pair1_asks[0][1])) * 0.001
                else:
                    if float(pair1_asks[0][1]) >= 5:
                        coin_amount = (float(pair1_asks[0][0]) / starting_amount_USD * 0.001) * 0.001 # Accounts for purchases from UTDT to USDC ex.
                    else: # if pair1_bids < 5
                        coin_amount = (float(pair1_asks[0][0]) / float(pair1_asks[0][1])) * 0.001
            coin_amount = (int(coin_amount) + int(np.format_float_positional(coin_amount, trim="-").split(".")[1][:8]) / 100000) # math.floor rounds down, math.ceil round up
            if coin_amount == 0:
                continue
            
            # Transaction 2
            if where_is_transaction_coin_two[1] == 2:
               if float(pair2_bids[0][1]) >= 5:
                    coin_amount = (float(pair2_bids[0][0]) / starting_amount_USD) * 0.001
               else: # if pair1_bids < 5
                    coin_amount = (float(pair2_bids[0][0]) / float(pair2_bids[0][1])) * 0.001

            if where_are_stable_coins[1] == 3:
                if float(pair2_asks[0][1]) >= 5:
                    coin_amount = (float(pair2_asks[0][0]) / starting_amount_USD) * 0.001
                else: # if pair1_bids < 5
                    coin_amount = (float(pair2_asks[0][0]) / float(pair2_asks[0][1])) * 0.001
            coin_amount = (int(coin_amount) + int(np.format_float_positional(coin_amount, trim="-").split(".")[1][:8]) / 100000) # math.floor rounds down, math.ceil round up
            if coin_amount == 0:
                continue

            # Transaction 3
            if where_is_transaction_coin_three[1] == 4:
               if float(pair3_bids[0][1]) >= 5:
                    coin_amount = (float(pair3_bids[0][0]) / starting_amount_USD) * 0.001
               else: # if pair1_bids < 5
                    coin_amount = (float(pair3_bids[0][0]) / float(pair3_bids[0][1])) * 0.001

            if where_are_stable_coins[1] == 5:
                if float(pair3_asks[0][1]) >= 5:
                    coin_amount = (float(pair3_asks[0][0]) / starting_amount_USD) * 0.001
                else: # if pair1_bids < 5
                    coin_amount = (float(pair3_asks[0][0]) / float(pair3_asks[0][1])) * 0.001
            coin_amount = (int(coin_amount) + int(np.format_float_positional(coin_amount, trim="-").split(".")[1][:8]) / 100000) # math.floor rounds down, math.ceil round up            
            if coin_amount == 0:
                continue

            print(starting_amount_USD, coin_amount)
    # Logic to determine if a path is availibe


def find_arb():
    while True:
        if find_tri_arb_path():
            pass
            # Print results to a loging api - work on actually trading later!!
            # Logic to execute trade with path

if __name__ == "__main__":
    import time
    start_time = time.time() 
    find_tri_arb_path()
    print(f'Algo took {str(time.time() - start_time)[:2]} seconds')
