from itertools import combinations
from kucoin.client import Market
from threading import Thread
import json
import subprocess
import os


default_coin = "USDT"
stable_coins = ["USDT", "TUSD", "BUSD", "USDC", "DAI"] #"PAX"
kucoin_fees = 0.1


client = Market(url="https://api.kucoin.com")


def get_tradable_coin_pairs():
    coin_pairs = []
    count = 0
    for i in client.get_symbol_list():
        if i["enableTrading"]:
            coin_pairs.append(i["symbol"])
    return coin_pairs


def get_all_coins():
    coins = []
    for i in client.get_currencies():
        coins.append(i["currency"])
    return coin


def find_tri_arb_path():
    pairs_in_Results = os.listdir(f"{os.getcwd()}/Results/")
    
    for combo in combinations(pairs_in_Results, 3):
        with open(f"{os.getcwd()}/Results/{combo[0]}") as f:
            pair1_orderbook = json.loads(f.read())
        with open(f"{os.getcwd()}/Results/{combo[1]}") as f:
            pair2_orderbook = json.loads(f.read())
        with open(f"{os.getcwd()}/Results/{combo[2]}") as f:
            pair3_orderbook = json.loads(f.read())

        pair1 = list(pair1_orderbook.keys())[0]
        pair2 = list(pair2_orderbook.keys())[0]
        pair3 = list(pair3_orderbook.keys())[0]

        # Needs to pass two requirements:
        # 1) 2 pairs need to have stable coins.
        # 2) I need to be able to chain together the 3 pairs BTC->ETH->KCS->BTC
        
        stable_coins_check = False
        right_order_check = False
        pairs_chain_check = False
        pairs_list = [pair1.split("-")[0], pair1.split("-")[1], pair2.split("-")[0], pair2.split("-")[1], pair3.split("-")[0], pair3.split("-")[1]]

        for i in stable_coins:
            if pairs_list.count(i) == 2:
                stable_coin_in_pairs = i
                stable_coins_check = True
                break

        # Ensures the beginning and end of pairs_list are both stable coins
        if (i == pairs_list[0] or i == pairs_list[1] and
            i == pairs_list[4] or i == pairs_list[5]):
            right_order_check = True

        if (pair1.split("-")[0] == pair2.split("-")[0] or 
            pair1.split("-")[0] == pair2.split("-")[1] or 
            pair1.split("-")[1] == pair2.split("-")[0] or 
            pair1.split("-")[1] == pair2.split("-")[1] and
            pair2.split("-")[0] == pair3.split("-")[0] or
            pair2.split("-")[0] == pair3.split("-")[1] or
            pair2.split("-")[1] == pair3.split("-")[0] or
            pair2.split("-")[1] == pair3.split("-")[1]):
            pairs_chain_check = True
                    
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
            for index, item in enumerate(pairs_list):
                if where_are_stable_coins[0] == 0:
                    if item == pairs_list[where_are_stable_coins[1]]:
                        where_is_transaction_coin_two.append(index)            
                if where_are_stable_coins[0] == 1:
                    if item == pairs_list[where_are_stable_coins[0]]
                        where_is_transaction_coin_two.append(index)

            # Transaction 3
            where_is_transaction_coin_three = [] # [3, 5]
            for index, item in enumerate(pairs_list):
                if item == pairs_list[where_are_stable_coins[1]]
                    where_is_transaction_coin_two.append(index)

            
            # Calculates if price is feasible
            coin_amount = 0
            starting_amount_USD = # needs to be the smallest (ask or bid) in all the pair files 
            smallest_amount = 0 # This is the smallest order in the chain and will be the amount I trade
            if where_are_stable_coins[0] == 0:
                if where_are_stable_coins[0] == "USDT"
                    if pair1_bids[0][1] >= 5:
                        coin_amount = (pair1_bids[0][0] / starting_amount_USD) * 0.001
                    else: # if pair1_bids < 5
                        coin_amount = (pair1_bids[0][0] / pair1_bids[0][1]) * 0.001
                    coin_amount = print(int(coin_amount) + int(str(coin_amount).split(".")[1][:8] / 100000000) # math.floor rounds down, math.ceil round up
                else:
                    if pair1_bids[0][1] >= 5:
                        coin_amount = (pair1_bids[0][0] / starting_amount_USD * 0.001) * 0.001 # Accounts for purchases from UTDT to USDC ex.
                    else: # if pair1_bids < 5
                        coin_amount = (pair1_bids[0][0] / pair1_bids[0][1]) * 0.001
                    coin_amount = print(int(coin_amount) + int(str(coin_amount).split(".")[1][:8] / 100000000) # math.floor rounds down, math.ceil round up

            if where_are_stable_coins[0] == 1:
                    if pair1_asks[0][1] >= 5:
                        coin_amount = (pair1_asks[0][0] / starting_amount_USD) * 0.001
                    else: # if pair1_bids < 5
                        coin_amount = (pair1_asks[0][0] / pair1_asks[0][1]) * 0.001
                    coin_amount = print(int(coin_amount) + int(str(coin_amount).split(".")[1][:8] / 100000000) # math.floor rounds down, math.ceil round up
                else:
                    if pair1_asks[0][1] >= 5:
                        coin_amount = (pair1_asks[0][0] / starting_amount_USD * 0.001) * 0.001 # Accounts for purchases from UTDT to USDC ex.
                    else: # if pair1_bids < 5
                        coin_amount = (pair1_asks[0][0] / pair1_asks[0][1]) * 0.001
                    coin_amount = print(int(coin_amount) + int(str(coin_amount).split(".")[1][:8] / 100000000) # math.floor rounds down, math.ceil round up
            
            # Transaction 2
            if where_is_transaction_coin_two[0] == 2:
                
            if where_is_transaction_coin_two[0] == 3:
 
    # Logic to determine if a path is availibe


def find_arb():
    while True:
        if find_tri_arb_path():
            pass
            # Print results to a loging api - work on actually trading later!!
            # Logic to execute trade with path


def thread_the_process(counter, coin_pairs_string):
    p = subprocess.Popen(["python", f"{os.getcwd()}/websockets.py", f"{counter}", f"{coin_pairs_string}"])
    p.wait()


if __name__ == "__main__":
    if not os.path.exists(f"{os.getcwd()}/Results/"):
        os.makedirs(f"{os.getcwd()}/Results/")

    process_list = []
    coin_pairs = get_tradable_coin_pairs()
    coin_pairs = [coin_pairs[i : i + 100] for i in range(0, len(coin_pairs), 100)]
    counter = 0

    for i in coin_pairs:
        counter = counter + 1
        coin_pairs_string = ""
        for o in i:
            coin_pairs_string += f"{o},"
        coin_pairs_string = coin_pairs_string[0:-1]
        
        # Runs websockets in a thread loop forever
        Thread(target=thread_the_process, args=(counter, coin_pairs_string)).start()

    # Determines if there is an Arbitrage
    #Thread(target=find_arb, args=()).Start()    
    #find_tri_arb_path()








