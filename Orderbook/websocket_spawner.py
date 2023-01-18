from itertools import combinations
from kucoin.client import Market
from threading import Thread
import json
import subprocess
import os
from path_finder_algo import find_tri_arb_path

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

