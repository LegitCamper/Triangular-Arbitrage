# Will execute trades when finds arbitrages

from tenacity import retry
import os

FIFO = f'{os.getcwd()}/trades.pipe'

try:
    os.mkfifo(FIFO)
except:
    os.remove(FIFO)
    os.mkfifo(FIFO)

api_key = '63b103c041a5330001d22229'
api_secret = '5b966871-aa93-4507-b31e-041606ca2fad'
api_passphrase = '@^uYR*FygYlnVR24fBq6srQbKq2kKNDh'

# Trade
from kucoin.client import Trade
#client = Trade(key='', secret='', passphrase='', is_sandbox=False, url='') # Real
client = Trade(api_key, api_secret, api_passphrase, is_sandbox=True) # # Sandbox

@retry(stop=(stop_after_delay(10) | stop_after_attempt(5)))
def make_order(data):
    # Place order with the following arguments Pair, Buy/Sell, Amount, Price
    client.create_limit_order(data[0], data[1], float(data[2]), float(data[3]))

# place a limit buy order
with open(FIFO) as fifo:
    while True:
        data = fifo.read()
        if data != "":
            
            data = data.split(" ")

            make_order(data)
