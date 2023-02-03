from kucoin.client import Market
import numpy as np
import json
import os
import shutil
import time


stable_coins = ["USDT", "TUSD", "BUSD", "USDC", "DAI"]

pair_catalog_file = open(f"{os.getcwd()}/Triangular_pairs.catalog", "r")

starting_amount_USD = 20

pair_catalog = json.load(pair_catalog_file)

client = Market(url="https://api.kucoin.com")
pair_data = client.get_symbol_list()


def round_value(coin_amount, **kwargs):
    if "pair" in kwargs.keys():
        # Double checks all data is good
        for i in pair_data:
            if kwargs['pair'] == i['symbol']:
                # Ensures size is not over or under requirement
                if coin_amount < float(i['baseMaxSize']):
                    coin_amount = coin_amount
                else:
                    coin_amount = float(i['baseMaxSize'])
                #if coin_amount > float(i['baseMinSize']):
                #    coin_amount = coin_amount
                #else:
                #    coin_amount = float(i['baseMinSize']) # this should mean it requires more than I have !!!!!
                #    print("tried to buy coin under the limit")
                #    raise

                # Ensurs rounds to the right decimal place
                length = len(i['baseIncrement'].split(".")[1])
    else:
        length = 8

    if coin_amount == None or coin_amount == 0.0:
        return 0.0
    scientific_to_decimal = np.format_float_positional(coin_amount, trim='-')
    split_value = scientific_to_decimal.split(".")
    if len(split_value) == 1:
        return float(split_value[0])
    return float(f'{split_value[0]}.{split_value[1][:length]}') # math.floor rounds down, math.ceil round up


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

            try: # Fixes index error
                # Calculations
                # Transaction 1
                if where_are_stable_coins[0] == 0:
                    coin_amount = starting_amount_USD * float(pair1_bids[0][0])
                elif where_are_stable_coins[0] == 1:
                    coin_amount = starting_amount_USD / float(pair1_asks[0][0])
                coin_amount = round_value(coin_amount, pair1)

                # Transaction 2
                if where_is_transaction_coin_two[1] == 2:
                    coin_amount = coin_amount * float(pair2_bids[0][0])
                elif where_is_transaction_coin_two[1] == 3:
                    coin_amount = coin_amount / float(pair2_asks[0][0])
                coin_amount = round_value(coin_amount, pair2)

                # Transaction 3
                if where_is_transaction_coin_three[1] == 4:
                    coin_amount = coin_amount * float(pair3_bids[0][0])
                elif where_is_transaction_coin_three[1] == 5:
                    coin_amount = coin_amount / float(pair3_asks[0][0])
                coin_amount = round_value(coin_amount, pair3)

                # Transaction 4 - If need to exchange back to USDT
                if where_are_stable_coins[0] != 'USDT':
                    coin_amount = round_value(coin_amount - (coin_amount * 0.012)) # 0.5% fees
                else:
                    coin_amount = round_value(coin_amount - (coin_amount * 0.009)) # 0.3% fees
            except:
                continue

            if (coin_amount - starting_amount_USD) > 0.01:
                print(f"\n For pair: {pairs}\nI now have {coin_amount}\nWhich means a net of ${coin_amount-starting_amount_USD}")
                if "USDT" in pair1: # It starts with USDT so its easy
                    
                    try:
                        if where_are_stable_coins[0] == 0:
                            os.system(f"echo '{pair1} sell {round_value(starting_amount_USD * float(pair1_bids[0][0]), pair=pair1)} {pair1_bids[0][0]}' >> {os.getcwd()}/trades.pipe")
                            coin_amount = starting_amount_USD * float(pair1_bids[0][0])
                        elif where_are_stable_coins[0] == 1:
                            os.system(f"echo '{pair1} buy {round_value(starting_amount_USD / float(pair1_asks[0][0]), pair=pair1)} {pair1_asks[0][0]}' >> {os.getcwd()}/trades.pipe")
                            coin_amount = starting_amount_USD / float(pair1_asks[0][0])
            
                        if where_is_transaction_coin_two[1] == 2:
                            os.system(f"echo '{pair2} sell {round_value((coin_amount * float(pair2_bids[0][0]) * 0.003), pair=pair2)} {pair2_bids[0][0]}' >> {os.getcwd()}/trades.pipe")
                            coin_amount = coin_amount * float(pair2_bids[0][0])
                        elif where_is_transaction_coin_two[1] == 3:
                            os.system(f"echo '{pair2} buy {round_value((coin_amount / float(pair2_asks[0][0]) * 0.003), pair=pair2)} {pair2_asks[0][0]}' >> {os.getcwd()}/trades.pipe")
                            coin_amount = coin_amount / float(pair2_asks[0][0])

                        if where_is_transaction_coin_three[1] == 4:
                            os.system(f"echo '{pair3} sell {round_value((coin_amount * float(pair3_bids[0][0]) * 0.003), pair=pair3)} {pair3_bids[0][0]}' >> {os.getcwd()}/trades.pipe")
                        elif where_is_transaction_coin_three[1] == 5:
                            os.system(f"echo '{pair3} buy {round_value((coin_amount / float(pair2_asks[0][0]) * 0.003), pair=pair3)} {pair3_asks[0][0]}' >> {os.getcwd()}/trades.pipe")
                    except:
                        continue


if __name__ == "__main__":
    while True:
        find_tri_arb_path()
