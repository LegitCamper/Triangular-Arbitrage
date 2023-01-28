# This script is intended to be ran alone. It will index all the pairs and find the chainable ones
# It takes quite a while to run so you can compile and run it with:
# nuitka3-run --quiet --remove-output --output-dir=bin --clang create_valid_pairs_catalog.py
import queue
from kucoin.client import Market
from threading import Thread
from queue import Queue
import multiprocessing
import json
import os
import time

stable_coins = ["USDT", "TUSD", "BUSD", "USDC", "DAI"]

pairs_catalog_path = f"{os.getcwd()}/Triangular_pairs.catalog"
if os.path.exists(pairs_catalog_path):
    os.remove(pairs_catalog_path)
pairs_catalog = open(pairs_catalog_path, 'w')

catalog_output = []

num_threads = multiprocessing.cpu_count()
thread_queue = Queue(maxsize=num_threads)

client = Market(url="https://api.kucoin.com")


def get_tradable_coin_pairs():
    coin_pairs = []
    for i in client.get_symbol_list():
        if i["enableTrading"]:
            coin_pairs.append(i["symbol"].split("-"))
    return coin_pairs

coin_pairs = get_tradable_coin_pairs()

def valid_combination_3(pair1):
    # Needs to pass three requirements:
    # 1) 2 pairs need to have stable coins.
    # 2) the stable coins must only be in the first and third pair
    # 3) I need to be able to chain together the 3 pairs USDT-BTC->BTC-ETH->ETH-USDT
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

                        catalog_output.append(pairs_list)
                except:
                    pass

# Probably will never need thisTriangular_pairs.catalog
def valid_combination_4(pair1, pair2):
    for pair3 in coin_pairs:
        for pair4 in coin_pairs:

            pairs_list = [pair1[0], pair1[1], pair2[0], pair2[1], pair3[0], pair3[1], pair4[0], pair4[1]]

            # Ensure the pairs can chain together
            if (pairs_list.count(pairs_list[0]) == 2 and
                pairs_list.count(pairs_list[1]) == 2 and
                pairs_list.count(pairs_list[2]) == 2 and
                pairs_list.count(pairs_list[3]) == 2 and
                pairs_list.count(pairs_list[4]) == 2 and
                pairs_list.count(pairs_list[5]) == 2):

                # First and last pair have a stable coin
                for i in stable_coins:
                    if i in pair1 and i in pair4:
                        i_ = i
                        
                try:
                    # Ensures the beginning and end of pairs_list are both stable coins
                    if (i_ == pairs_list[0] or i_ == pairs_list[1] and
                        i_ == pairs_list[6] or i_ == pairs_list[7]):

                            catalog_output.append(pairs_list)
                except:
                    pass

def create_catalog():
    # Creats all valid combinations with 3 pairs
    for pair1 in coin_pairs:
    
        try:
            # ensurs the queue always stays full
            while True:
                thread_queue.put(Thread(target=valid_combination_3, args=(pair1,), daemon=True).start(), block=False)
        except queue.Full:
            pass 
        except queue.Empty:
            print('finished')

        #for pair2 in coin_pairs:

        #    try:
        #        # ensurs the queue always stays full
        #        while True:
        #            thread_queue.put(Thread(target=valid_combination_4, args=(pair1, pair2,), daemon=True).start(), block=False)
        #    except queue.Full:
        #        pass 
        #    except queue.Empty:
        #        print('finished')
     

    # Writes the results to Triangular_pairs.catalog
    json.dump(catalog_output, pairs_catalog)

if __name__ == "__main__":
    print('This will create the pair catalog (takes a couple minutes to run)')
    start_time = time.time()
    create_catalog()
    print(f"Creating the catalog took {(round(time.time() - start_time, 2)) / 60} minutes")

    #print("\nbelow is the number of unique coins in catalog (all coins in kucoin is 1247)")
    #print(count_coins_in_catalog())
