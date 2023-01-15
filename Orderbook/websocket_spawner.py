from itertools import combinations
from kucoin.client import Market
from threading import Thread
import json
import subprocess
import os


default_currancy = "USDT"
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
        
        coin_counter = 0
        for i in stable_coins:
            if (par1.split("-")[0] == i and part1.split()


                )

        if (pair1.split("-")[0] == pair2.split("-")[0] or 
            pair1.split("-")[0] == pair2.split("-")[1] or 
            pair1.split("-")[1] == pair2.split("-")[0] or 
            pair1.split("-")[1] == pair2.split("-")[1] and
            pair2.split("-")[0] == pair3.split("-")[0] or
            pair2.split("-")[0] == pair3.split("-")[1] or
            pair2.split("-")[1] == pair3.split("-")[0] or
            pair2.split("-")[1] == pair3.split("-")[1]):

            



            for i in stable_coins:
                if i != pair1.split("-")[0]:
                    Pairs_okay = False
                    break
                else:
                    Pairs_okay = True






                    )

        Pairs_okay = False 
        if Pairs_okay:

        pair1_asks = pair1_orderbook[pair1]['asks']
        pair1_bids = pair1_orderbook[pair1]['bids']
        pair2_asks = pair2_orderbook[pair2]['asks']
        pair2_bids = pair2_orderbook[pair2]['asks']

        # MANA-BTC
        # Price in BTC, Amount in MANA 
        # Bid is the price buyers are ready to buy at
        # Ask id the price sellers are ready to sell at



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
        #Thread(target=thread_the_process, args=(counter, coin_pairs_string)).start()

    # Determines if there is an Arbitrage
    #Thread(target=find_arb, args=()).Start()    
    find_tri_arb_path()

