#from filelock import Timeout, FileLock
import websocket, ssl
import _thread
import time
import rel
import sys
import requests
import random
import json
import os


def write_coin_pairs(pair, data):
    with open(f"{os.getcwd()}/Results/{pair}.kupair", "w") as f:
        json.dump(data, f)


def read_coin_pairs(pair):
    try:
        with open(f"{os.getcwd()}/Results/{pair}.kupair", "r") as f:
            return json.loads(f.read())
    except:
        return {}

def get_new_proxy():
    proxies_url = 'https://api.proxyscrape.com/v2/?request=displayproxies&protocol=http&timeout=10000&country=us&ssl=sll&anonymity=all'
    response = requests.get(proxies_url).text.split("\r\n")
    for i in range(len(response)):
        response[i] = response[i].split(":")
    return response[:-1]
proxies = get_new_proxy()


class kucoin_orderbook_websocket():
    def __init__(self, threadnumber, pair_strings):
        self.threadnumber = threadnumber
        self.proxy = proxies[random.randint(0, len(proxies)-1)]
        self.kucoin_auth = requests.post('https://api.kucoin.com/api/v1/bullet-public', proxies={'http': f'http://{self.proxy[0]}:{self.proxy[1]}'}).json()['data']
        self.endpoint_data = self.kucoin_auth['instanceServers'][0]
        self.pair_strings = pair_strings

        self.Start()

    def on_message(self, ws, message):
        msg = json.loads(message)
        if 'topic' in msg.keys():
            pair = msg["topic"].split(":")[1]
            coin_pair_prices = read_coin_pairs(pair) 
            if len(coin_pair_prices.keys()) >= 1:
                if msg["data"]["timestamp"] > coin_pair_prices[pair]["timestamp"]:
                    coin_pair_prices[pair] = msg["data"]
            else:
                coin_pair_prices[pair] = msg["data"]
            write_coin_pairs(pair, coin_pair_prices)


    def on_error(self, ws, error):
        print(f"Thread Number: {self.threadnumber}", "ERROR:", error)
        ws.close()  
        self.__init__(self.pair_strings, self.first_run)

    def on_close(self, ws, close_status_code, close_msg):
        print("### closed ###", f"Thread Number: {self.threadnumber}")

    def on_open(self, ws):
        depth=5
        print(f"Thread Number: {self.threadnumber}", "Opened connection")
        ws.send(str({"id": random.randint(1000000000000, 9999999999999),
                "type": "subscribe",
                "topic": f"/spotMarket/level2Depth{depth}:{self.pair_strings}",
                "privateChannel": "false",
                "response": "false"}))

    def Start(self):
        websocket.enableTrace(False)
        ws = websocket.WebSocketApp(f"{self.endpoint_data['endpoint']}/market/level2:{self.pair_strings}?token={self.kucoin_auth['token']}",
                              on_open=self.on_open,
                              on_message=self.on_message,
                              on_error=self.on_error,
                              on_close=self.on_close)

        ws.run_forever(proxy_type="http", http_proxy_timeout=self.endpoint_data['pingTimeout']/1000, 
                       #http_proxy_host=self.proxy[0], http_proxy_port=self.proxy[1], 
                       dispatcher=rel, reconnect=5,
                       sslopt={"cert_reqs": ssl.CERT_NONE, "check_hostname": False},
                       ping_interval=self.endpoint_data['pingInterval']/1000, 
                       ping_timeout=self.endpoint_data['pingTimeout']/1000) 
        rel.signal(2, rel.abort)  # Keyboard Interrupt
        rel.dispatch()


if __name__ == "__main__":
    # Ensure arguments follow this order threadnumber stringofcoins
    kucoin_orderbook_websocket(sys.argv[1], sys.argv[2])
