# Will execute trades when finds arbitrages

from random import randint
from tenacity import retry
from tenacity.stop import stop_after_attempt
import requests
requests.packages.urllib3.util.connection.HAS_IPV6 = False
import json
import time
import os

import hmac
import hashlib
import base64

FIFO = f'{os.getcwd()}/trades.pipe'

def new_fifo():
    try:
        os.mkfifo(FIFO)
    except:
        os.remove(FIFO)
        os.mkfifo(FIFO)

    return open(FIFO, "r")

# Get API Keys
with open(f"{os.getcwd()}/KucoinKeys.json") as f:
    keys = json.load(f)
    api_key = keys['kucoinApiKey']
    api_secret = keys['kucoinApiSecret']
    api_passphrase = keys['kucoinApiPassphrase']
    api_passphrase = base64.b64encode(hmac.new(api_secret.encode('utf-8'), api_passphrase.encode('utf-8'), hashlib.sha256).digest())

# Trade
s = requests.Session()
def get_login():
    return{"KC-API-KEY": api_key, "KC-API-PASSPHRASE": api_passphrase, "KC-API-KEY-VERSION": "2", "KC-API-TIMESTAMP": str(int(round(time.time(), 3) * 1000))} # Login details

restricted_pairs = []

#@retry(stop=(stop_after_attempt(5)))
def make_order(data):
    url_endpoint = '/api/v1/orders'

    if data[0] not in restricted_pairs:
        post_headers = get_login()

        post_data = {"symbol": data[0], "side": data[1], "type": "limit", "size": data[2], "price": data[3], "timeInForce": "IOC", "clientOid": randint(1000, 99999)}

        str_to_sign = str(int(round(time.time(), 3) * 1000)) + 'POST' + url_endpoint + json.dumps(post_data)
        api_signature = base64.b64encode(hmac.new(api_secret.encode('utf-8'), str_to_sign.encode('utf-8'), hashlib.sha256).digest())

        post_headers["KC-API-SIGN"] = api_signature
        req = s.post(f'https://api.kucoin.com{url_endpoint}', headers=post_headers, json=post_data).json()

        if req['code'] != '200000':
            raise Exception(req)

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
                # Error handling and retries
                try:
                    make_order(data)

                except Exception as e:
                    e = str(e)
                    if "403" in e:
                        time.sleep(10)
                    elif "Not Exists" in e:
                        print("Not Exists")
                    elif '400500' in e:
                        restricted_pairs.append(data[0])
                    else:
                        print(e)

    fifo.close() # Allows the fifo to be deleted and re-created
