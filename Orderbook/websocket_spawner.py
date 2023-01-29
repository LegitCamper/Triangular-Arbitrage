#from kucoin.client import Market
import json
from threading import Thread
import subprocess
import os

default_coin = "USDT"
stable_coins = ["USDT", "TUSD", "BUSD", "USDC", "DAI"] #"PAX"

#client = Market(url="https://api.kucoin.com")


#def get_tradable_coin_pairs():
#    coin_pairs = []
#    for i in client.get_symbol_list():
#        if i["enableTrading"]:
#            coin_pairs.append(i["symbol"])
#    return coin_pairs


#def get_all_coins():
#    coins = []
#    for i in client.get_currencies():
#        coins.append(i["currency"])
#    return coins


# Only download the coins that I will be using
# AKA the ones in Triangular_pair.catalog
def get_tradable_coin_pairs():
    file = open(f"{os.getcwd()}/Triangular_pairs.catalog", "r")
    coin_pairs = []
    for i in json.load(file):
        coin_pairs.append(f"{i[0]}-{i[1]}")
        coin_pairs.append(f"{i[2]}-{i[3]}")
        coin_pairs.append(f"{i[4]}-{i[5]}")
    return coin_pairs


def thread_the_process(counter, coin_pairs_string):
    p = subprocess.Popen([f"{os.getcwd()}/websockets.bin", f"{counter}", f"{coin_pairs_string}"])
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

