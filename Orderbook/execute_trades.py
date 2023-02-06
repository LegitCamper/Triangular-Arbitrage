# Will execute trades when finds arbitrages

from tenacity import retry
from tenacity.stop import stop_after_attempt
import time
import os

FIFO = f'{os.getcwd()}/trades.pipe'

def new_fifo():
    try:
        os.mkfifo(FIFO)
    except:
        os.remove(FIFO)
        os.mkfifo(FIFO)

    return open(FIFO, "r")

api_key = '63dacd030d23f70001d0a924'
api_secret = '19756bc7-4504-4072-873e-ccc4e1d4ad9f'
api_passphrase = '@^uYR*FygYlnVR24fBq6srQbKq2kKNDh'

# Trade
from kucoin.client import Trade
client = Trade(key=api_key, secret=api_secret, passphrase=api_passphrase, is_sandbox=False, url='') # Real
#client = Trade(api_key, api_secret, api_passphrase, is_sandbox=True) # Sandbox

restricted_pairs = []

#@retry(stop=(stop_after_attempt(5)))
def make_order(data):
    try:
        if data[0] not in restricted_pairs:
            # Place order with the following arguments Pair, Buy/Sell, Amount, Price
            client.create_limit_order(data[0], data[1], float(data[2]), float(data[3]), timeInForce="FOK")
    except Exception as e:
        if "403" in str(e):
            time.sleep(10)
        elif "Not Exists" in str(e):
            print("Not Exists")
        elif "Your located country/region is currently not supported for the trading of this token" in str(e):
            restricted_pairs.append(data[0])
            pass
        else:
            print(e)

# place a limit buy order
while True:
    fifo = new_fifo()
    for line in fifo:
        if line != "":

            line = line.replace("[", "")
            line = line.replace("]", "")
            line = line.replace("\n", "")
            line = line.split(", ")
            for data in line:
                data = data.split(" ")

                print(data)
                make_order(data)
                #time.sleep(0.2) # waits for order to fill before executing a new one

    fifo.close() # Allows the fifo to be deleted and re-created
