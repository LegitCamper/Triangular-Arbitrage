# Will execute trades when finds arbitrages

import sys

# I ideally would use websocets, but i'll use rest for now

api_key = '63b103c041a5330001d22229'
api_secret = '5b966871-aa93-4507-b31e-041606ca2fad'
api_passphrase = '@^uYR*FygYlnVR24fBq6srQbKq2kKNDh'

# Trade
from kucoin.client import Trade
#client = Trade(key='', secret='', passphrase='', is_sandbox=False, url='') # Real
client = Trade(api_key, api_secret, api_passphrase, is_sandbox=True) # # Sandbox

# place a limit buy order
#                               Place order with the following arguments Pair, Buy/Sell, Amount, Price
order_id = client.create_limit_order(sys.argv[1], sys.argv[2], float(sys.argv[3]), float(sys.argv[4]))

