from kucoin.client import Market
import numpy as np
import json
import os
import shutil
import time


pair_catalog_file = open(f"{os.getcwd()}/Triangular_pairs.catalog", "r")
pair_catalog = json.load(pair_catalog_file)

starting_amount_USD = 20
USING_KCS_FOR_FEES = True


client = Market(url="https://api.kucoin.com")
pair_data = client.get_symbol_list()


oldest_order_in_pairs = 10 # seconds
stable_coins = ["USDT", "TUSD", "BUSD", "USDC", "DAI"]
# Fees are always calculated for the coins on the left of the pair. For example, for KCS/BTC, KCS/ETH, and KCS/USDT, fees are calculated based on KCS.
coin_fees = {"Class A": {"Regular Maker": 0.001, "Regular Taker": 0.001, "KCS Maker": 0.0008, "KCS Taker": 0.0008, "Coins": ["1INCH", "AAVE", "AAVE3L", "AAVE3S", "ACH", "ACOIN", "ACT", "ADA", "ADA3L", "ADA3S", "ADB", "ADX", "AERGO", "AGIX", "AGLD", "AION", "AKRO", "ALEPH", "ALGO", "ALICE", "ALPA", "ALPHA", "ALPINE", "AMB", "AMP", "AMPL", "ANC", "ANKR", "ANT", "AOA", "APE", "APE3L", "APE3S", "API3", "APT", "AR", "ARPA", "ARX", "ASD", "ASTR", "ASTROBOY", "ATA", "ATOM", "ATOM3L", "ATOM3S", "AUDIO", "AURORA", "AVA", "AVAX", "AVAX3L", "AVAX3S", "AXE", "AXPR", "AXS", "AXS3L", "AXS3S", "AZERO", "BADGER", "BAKE", "BAL", "BAND", "BAT", "BAX", "BCD", "BCH", "BCH3L", "BCH3S", "BCHSV", "BEPRO", "BETA", "BICO", "BNB", "BNB3L", "BNB3S", "BNS", "BNT", "BNX", "BOBA", "BOLT", "BOND", "BRWL", "BSW", "BTC", "BTC3L", "BTC3S", "BTCP", "BTT", "BURGER", "BUSD", "BUX", "BUY", "C98", "CADH", "CAKE", "CAPP", "CELO", "CELR", "CELT", "CFX", "CHR", "CHZ", "CIX100", "CKB", "CLUB", "CLV", "COCOS", "COMP", "COOHA", "COS", "COTI", "COV", "CPC", "CREAM", "CRO", "CRPT", "CRV", "CS", "CTC", "CTSI", "CV", "CVC", "CVX", "DAG", "DAI", "DAPPT", "DAR", "DASH", "DATA", "DCR", "DEGO", "DENT", "DERO", "DEXE", "DFA", "DGB", "DIA", "DLTA", "DOCK", "DODO", "DOGE", "DOGE3L", "DOGE3S", "DON", "DOSE", "DOT", "DOT3L", "DOT3S", "DRGN", "DUSK", "DYDX", "EGLD", "ELA", "ELF", "ELITEHERO", "ENJ", "ENQ", "ENS", "EOS", "EOS3L", "EOS3S", "EOSC", "EPIK", "EPRX", "EPX", "ERN", "ERTHA", "ETC", "ETF", "ETH", "ETH2", "ETH3L", "ETH3S", "ETHW", "ETN", "EUL", "EWT", "EXRD", "FET", "FIDA", "FIL", "FITFI", "FKX", "FLOW", "FLR", "FLUX", "FORESTPLUS", "FORT", "FORTH", "FRONT", "FSN", "FTM", "FTM3L", "FTM3S", "FTT", "FX", "FXS", "GAL", "GALAX3L", "GALAX3S", "GAS", "GGC", "GHST", "GLM", "GMB", "GMT", "GMT3L", "GMT3S", "GMX", "GO", "GOD", "GODS", "GRIN", "GRT", "GST", "GTC", "GZIL", "H2O", "HARD", "HBAR", "HEGIC", "HFT", "HNT", "ICP", "ICX", "IDEX", "IDLENFT", "ILV", "IMX", "INDI", "INJ", "IOST", "IOTA", "IOTX", "J8T", "JAR", "JASMY", "JASMY3L", "JASMY3S", "JST", "KAI", "KAR", "KAT", "KAVA", "KCANDY", "KEY", "KLAY", "KMA", "KMD", "KNC", "KOL", "KONO", "KP3R", "KSM", "KTS", "LBP", "LDO", "LINA", "LINK", "LINK3L", "LINK3S", "LIT", "LMR", "LOC", "LOKA", "LOKI", "LOL", "LOOKS", "LOOM", "LPT", "LRC", "LSK", "LTC", "LTC3L", "LTC3S", "LTO", "LUNA", "LUNA3L", "LUNA3S", "LUNC", "LYM", "LYXE", "MAGIC", "MAN", "MANA", "MANA3L", "MANA3S", "MAP", "MAP2", "MATIC", "MATIC3L", "MATIC3S", "MBL", "MBOX", "MDX", "METIS", "MFT", "MHC", "MKR", "MLN", "MNW", "MTL", "MTV", "MUSH", "MVP", "MXC", "MXW", "NEAR", "NEAR3L", "NEAR3S", "NEO", "NFT", "NFTB", "NGL", "NIM", "NKN", "NMR", "NOIA", "NRG", "NULS", "NVT", "NYM", "OCEAN", "OGN", "OGV", "OLT", "OM", "OMG", "ONE", "ONG", "ONT", "OOKI", "OP", "OPCT", "OPT", "ORBS", "ORN", "OSMO", "OXT", "PAXG", "PEOPLE", "PERP", "PHA", "PIKASTER", "PIKASTER2", "PIVX", "PLAY", "PNK", "PNT", "POLS", "POND", "POSI", "POWR", "PPT", "PRE", "PROM", "PSTAKE", "PUSH", "QKC", "QNT", "QTUM", "QUICK", "R", "RACA", "RBTC", "REEF", "REN", "REP", "REQ", "RFUEL", "RIF", "RLC", "ROOBEE", "RPL", "RSR", "RUNE", "RVN", "SAND", "SAND3L", "SAND3S", "SATT", "SCRT", "SENSO", "SFP", "SHA", "SHIB", "SHR", "SKL", "SKU", "SLP", "SMT", "SNT", "SNX", "SOL", "SOL3L", "SOL3S", "SOLVE", "SOUL", "SPA", "SPELL", "SPHRI", "SRM", "STEEM", "STMX", "STORE", "STORJ", "STRK", "STX", "SUKU", "SUN", "SUPER", "SUSD", "SUSHI", "SUSHI3L", "SUSHI3S", "SUTER", "SWEAT", "SWFTC", "SXP", "SYLO", "SYS", "T", "TEL", "TFUEL", "THETA", "TIME", "TITAN", "TKO", "TLM", "TOKO", "TOMO", "TON", "TONE", "TORN", "TRAC", "TRB", "TRIAS", "TRIBE", "TRU", "TRX", "TT", "TUSD", "TVK", "TWT", "UBX", "UBXT", "UDOO", "UMA", "UNFI", "UNI", "UNI3L", "UNI3S", "UQC", "USDC", "USDD", "USDJ", "USDN", "USDP", "USDT", "UST", "USTC", "UTK", "VET", "VET3L", "VET3S", "VI", "VID", "VIDT", "VOXEL", "VRA", "VSYS", "VTHO", "WAN", "WAVES", "WAX", "WBTC", "WEST", "WHALE", "WIN", "WNCG", "WNXM", "WOM", "WOO", "WRX", "WXT", "XCH", "XDB", "XEC", "XEM", "XETA", "XLM", "XMR", "XNO", "XPR", "XRACER", "XRD", "XRP", "XRP3L", "XRP3S", "XTZ", "XVS", "XYM", "XYO", "YFI", "YGG", "ZBC", "ZEC", "ZEN", "ZIL", "ZRX"]},
             "Class B": {"Regular Maker": 0.002, "Regular Taker": 0.002, "KCS Maker": 0.0016, "KCS Taker": 0.0016, "Coins": ["00", "1EARTH", "2CRZ", "ABBC", "ACA", "ACE", "ACQ", "ADS", "AFK", "AI", "AIOZ", "AKT", "ALBT", "ALPACA", "ALT", "AOG", "APL", "ARKER", "ARNM", "ARRR", "ASTRA", "AURY", "AUSD", "AXC", "BASIC", "BBC", "BDX", "BEAT", "BFC", "BIFI", "BLOK", "BMON", "BNC", "BOA", "BONDLY", "BOSON", "BRISE", "BULL", "CARD", "CARE", "CAS", "CCD", "CEEK", "CERE", "CEUR", "CFG", "CGG", "CIRUS", "CMP", "CPOOL", "CQT", "CREDI", "CSPR", "CTI", "CUDOS", "CULT", "CUSD", "CWEB", "CWS", "DAO", "DAPPX", "DC", "DERC", "DFI", "DFYN", "DG", "DIVI", "DMTR", "DORA", "DPET", "DPR", "DREAMS", "DSLA", "DVPN", "DYP", "ECOX", "EDG", "EFI", "EFX", "EGAME", "ELON", "EPK", "EQX", "EQZ", "ERG", "ERSDL", "ETHO", "EVER", "FALCONS", "FCD", "FCL", "FCON", "FEAR", "FLAME", "FLY", "FORM", "FORWARD", "FRA", "FRM", "FRR", "FTG", "GAFI", "GALAX", "GEEQ", "GEM", "GENS", "GGG", "GHX", "GLCH", "GLMR", "GLQ", "GMEE", "GMM", "GOM2", "GOVI", "H3RO3S", "HAI", "HAKA", "HAPI", "HAWK", "HBB", "HEART", "HERO", "HORD", "HT", "HTR", "HYDRA", "HYVE", "IDEA", "IHC", "ILA", "IOI", "ISP", "ITAMCUBE", "JAM", "JUP", "KARA", "KCS", "KDA", "KDON", "KICKS", "KLV", "KOK", "KRL", "KYL", "LABS", "LACE", "LAVAX", "LAYER", "LIKE", "LOCG", "LON", "LOVE", "LPOOL", "LSS", "LTX", "MAHA", "MAKI", "MARS4", "MARSH", "MASK", "MATCH", "MATTER", "MIR", "MITX", "MJT", "MLK", "MM", "MNET", "MNST", "MODEFI", "MONI", "MOOV", "MOVR", "MPLX", "MSWAP", "MTRG", "MTS", "MV", "NAKA", "NAVI", "NDAU", "NEER", "NGC", "NGM", "NHCT", "NORD", "NRFB", "NTVRK", "NUM", "OAS", "ODDZ", "OLE", "ONSTON", "OOE", "OPUL", "ORAI", "ORC", "OUSD", "OVR", "P00LS", "PBR", "PBX", "PCX", "PEEL", "PEL", "PHNX", "PIAS", "PIX", "PKF", "PLD", "PLGR", "PLU", "PMON", "POKT", "POL", "POLC", "POLK", "POLX", "PRIMAL", "PRMX", "PRQ", "PSL", "PUMLX", "PUNDIX", "PYR", "QI", "QRDO", "QUARTZ", "RACEFI", "RANKER", "RBP", "REAP", "RED", "REV3L", "REVU", "REVV", "RFOX", "RLY", "RMRK", "RNDR", "ROAR", "ROSE", "ROSN", "ROUTE", "RPC", "SCLP", "SDAO", "SDL", "SDN", "SFUND", "SHFT", "SHILL", "SHX", "SIENNA", "SIMP", "SIN", "SKEY", "SLCL", "SLIM", "SOLR", "SON", "SOS", "SOV", "SPI", "SQUAD", "SRBP", "SRK", "STARLY", "STC", "STEPWATCH", "STG", "STND", "STRONG", "SURV", "SWASH", "SWINGBY", "SWP", "SYNR", "TARA", "TAUM", "TCP", "TEM", "TIDAL", "TLOS", "TOWER", "TRADE", "TRIBL", "TRVL", "TXA", "UFO", "UNB", "UNIC", "UNO", "UOS", "UPO", "URUS", "VAI", "VEED", "VEGA", "VEMP", "VISION", "VLX", "VXV", "WAL", "WELL", "WEMIX", "WILD", "WMT", "WOMBAT", "WOOP", "WSIENNA", "XAVA", "XCAD", "XCN", "XCUR", "XCV", "XDC", "XDEFI", "XED", "XHV", "XNL", "XPRT", "XSR", "XTAG", "XTM", "XWG", "YFDAI", "YLD", "YOP", "ZCX", "ZEE", "ZKT"]},
             "Class C": {"Regular Maker": 0.003, "Regular Taker": 0.003, "KCS Maker": 0.0024, "KCS Taker": 0.0024, "Coins": ["BURP", "CHMB", "CLH", "COMB", "CWAR", "FT", "GARI", "HIAZUKI", "HIBAYC", "HIBIRDS", "HICLONEX", "HICOOLCATS", "HIDOODLES", "HIENS3", "HIENS4", "HIFIDENZA", "HIFLUF", "HIGAZERS", "HIMAYC", "HIMEEBITS", "HIMFERS", "HIOD", "HIODBS", "HIPENGUINS", "HIPUNKS", "HIRENGA", "HISAND33", "HISQUIGGLE", "HIVALHALLA", "HOTCROSS", "IXS", "LITH", "MC", "MELOS", "MEM", "MLS", "NWC", "PDEX", "PLY", "VELO", "VR"]}}


def calc_fees(pair):
    for i in pair_data:
        if pair == i['symbol']:
           feeCurrency = i['feeCurrency']
    if feeCurrency in coin_fees["Class A"]["Coins"]:
        fees_class = "Class A"
    elif feeCurrency in coin_fees["Class B"]["Coins"]:
        fees_class = "Class B"
    elif feeCurrency in coin_fees["Class C"]["Coins"]:
        fees_class = "Class C"

    try: # We are always a taker because we fill existing orders in the orderbook
        if USING_KCS_FOR_FEES:
            return coin_fees[fees_class]["KCS Taker"] 
        else:
            return coin_fees[fees_class]["Regular Taker"]
    except:
        return 0.003 # this is the worst possible fee so it cannot be worse - fixes case where fee currancy has no fee????

def round_value(coin_amount, **kwargs):
    if "pair" in kwargs.keys():
        # Double checks all data is good
        for i in pair_data:
            if kwargs['pair'] == i['symbol']:
                # Ensures size is not over or under requirement
                if coin_amount > float(i['baseMaxSize']):
                    coin_amount = float(i['baseMaxSize'])
                if coin_amount < float(i['baseMinSize']):
                    coin_amount = 0 #float(i['baseMinSize']) # this should mean it requires more than I have !!!!!

                # Ensurs rounds to the right decimal place
                length = len(i['baseIncrement'].split(".")[1])

    else:
        length = 8

    if coin_amount == None or coin_amount == 0.0:
        return 0.0

    scientific_to_decimal = np.format_float_positional(coin_amount, trim='-')
    split_value = scientific_to_decimal.split(".")
    if len(split_value) == 1:
        return float(split_value[0])
    return float(f'{split_value[0]}.{split_value[1][:length]}') # math.floor rounds down, math.ceil round up

def Read_File(path):
    shutil.copy(path, f"{os.getcwd()}/TempRead.kupair")

    with open(f"{os.getcwd()}/TempRead.kupair", "r") as f:
        return json.load(f)


def find_tri_arb_path():
    for pairs_list in pair_catalog:
               
        pair1 = f"{pairs_list[0]}-{pairs_list[1]}"
        pair2 = f"{pairs_list[2]}-{pairs_list[3]}"
        pair3 = f"{pairs_list[4]}-{pairs_list[5]}"

        try:
            pair1_orderbook = Read_File(f"{os.getcwd()}/Results/{pair1}.kupair")
            pair2_orderbook = Read_File(f"{os.getcwd()}/Results/{pair2}.kupair")
            pair3_orderbook = Read_File(f"{os.getcwd()}/Results/{pair3}.kupair")
        except:
            continue

        # Finds what the stable coin is 
        if pairs_list[0] == pairs_list[4]:
            stable_coin_in_pairs = pairs_list[0]
        if pairs_list[0] == pairs_list[5]:
            stable_coin_in_pairs = pairs_list[0]
        if pairs_list[1] == pairs_list[4]:
            stable_coin_in_pairs = pairs_list[1]
        if pairs_list[1] == pairs_list[5]:
            stable_coin_in_pairs = pairs_list[1]
             
        pair1_asks = pair1_orderbook[pair1]['asks']
        pair1_bids = pair1_orderbook[pair1]['bids']
        pair1_time = pair1_orderbook[pair1]['timestamp']
        pair2_asks = pair2_orderbook[pair2]['asks']
        pair2_bids = pair2_orderbook[pair2]['bids']
        pair2_time = pair2_orderbook[pair2]['timestamp']
        pair3_asks = pair3_orderbook[pair3]['asks']
        pair3_bids = pair3_orderbook[pair3]['bids']
        pair3_time = pair3_orderbook[pair3]['timestamp']

        # Ensures data is not too old
        now = int(time.time())
        if (now - int(str(pair1_time)[:-3])) > oldest_order_in_pairs or (now - int(str(pair2_time)[:-3])) > oldest_order_in_pairs or (now - int(str(pair3_time)[:-3])) > oldest_order_in_pairs:
            continue # data from one chain is older than a minute old and is no good

        
        # Transaction 1 Check
        where_are_stable_coins = [] # [0, 4]
        for index, item in enumerate(pairs_list):
            if item == stable_coin_in_pairs:
                where_are_stable_coins.append(index)

        # Transaction 2 Check
        where_is_transaction_coin_two = [] # [1, 2]
        if where_are_stable_coins[0] == 0:
            where_is_transaction_coin_two.append(1)
            where_is_transaction_coin_two.append(pairs_list.index(pairs_list[1], 2))
        elif where_are_stable_coins[0] == 1:
            where_is_transaction_coin_two.append(0)
            where_is_transaction_coin_two.append(pairs_list.index(pairs_list[0], 1))

        # Transaction 3 Check
        where_is_transaction_coin_three = []
        if where_is_transaction_coin_two[1] == 2:
            where_is_transaction_coin_three.append(3)
        elif where_is_transaction_coin_two[1] == 3:
            where_is_transaction_coin_three.append(2)
        if where_are_stable_coins[1] == 4:
            where_is_transaction_coin_three.append(5)
        elif where_are_stable_coins[1] == 5:
            where_is_transaction_coin_three.append(4)

       # MANA-BTC
        # Price in BTC, Amount in MANA 
        # Bid is the price buyers are ready to buy at
        # Ask id the price sellers are ready to sell at
        # The coin on the left is what I am either buying of selling

        # Calculations
        coin_amount = 0
        # Transaction 1
        if where_are_stable_coins[0] == 0:
            coin_amount = starting_amount_USD * float(pair1_bids[0][0])
            if coin_amount > float(pair1_bids[0][1]):
                continue
        elif where_are_stable_coins[0] == 1:
            coin_amount = starting_amount_USD / float(pair1_asks[0][0])
            if coin_amount > float(pair1_asks[0][1]):
                continue
        coin_amount = round_value(coin_amount - (coin_amount * calc_fees(pair1)), pair=pair1)
        if coin_amount == 0.0:
            continue

        # Transaction 2
        if where_is_transaction_coin_two[1] == 2:
            coin_amount = coin_amount * float(pair2_bids[0][0])
            if coin_amount > float(pair1_bids[0][1]):
                continue
        elif where_is_transaction_coin_two[1] == 3:
            coin_amount = coin_amount / float(pair2_asks[0][0])
            if coin_amount > float(pair2_asks[0][1]):
                continue
        coin_amount = round_value(coin_amount - (coin_amount * calc_fees(pair2)), pair=pair2)
        if coin_amount == 0.0:
            continue

        # Transaction 3
        if where_is_transaction_coin_three[1] == 4:
            coin_amount = coin_amount * float(pair3_bids[0][0])
            if coin_amount > float(pair3_bids[0][1]):
                continue
        elif where_is_transaction_coin_three[1] == 5:
            coin_amount = coin_amount / float(pair3_asks[0][0])
            if coin_amount > float(pair3_asks[0][1]):
                continue
        coin_amount = round_value(coin_amount - (coin_amount * calc_fees(pair3)), pair=pair3)
        if coin_amount == 0.0:
            continue

        # Transaction 4 - If need to exchange back to USDT - Work on later, for now focus on 3 pair chains
        #if where_are_stable_coins[0] != 'USDT':
        #    coin_amount = round_value(coin_amount - (coin_amount * 0.012)) # 0.12% fees

        if (coin_amount - starting_amount_USD) > 0.010:
            if "USDT" in pair1: # It starts with USDT so its easy

                pending_orders = [] 
                    
                if where_are_stable_coins[0] == 0:
                    coin_amount = round_value(starting_amount_USD * float(pair1_bids[0]), pair=pair1)
                    direction, price = "sell", pair1_bids[0]
                elif where_are_stable_coins[0] == 1:
                    direction, price = "buy", pair1_asks[0]
                    coin_amount = round_value(starting_amount_USD / float(pair1_asks[0]), pair=pair1)
                pending_orders.append(f'{pair1} {direction} {coin_amount} {price}')

                if where_is_transaction_coin_two[1] == 2:
                    #coin_amount = round_value(coin_amount * float(pair2_bids[0][0]), pair=pair2) # Dont need to calculate anything? If i already have 600 tokens I just want to sell 600 tokens
                    direction, price = "sell", pair2_bids[0]
                elif where_is_transaction_coin_two[1] == 3:
                    #coin_amount = round_value(coin_amount  / float(pair2_asks[0]), pair=pair2)
                    direction, price = "buy", pair2_asks[0]
                pending_orders.append(f'{pair1} {direction} {coin_amount} {price}')

                if where_is_transaction_coin_three[1] == 4:
                    coin_amount = round_value(coin_amount * float(pair3_bids[0]), pair=pair3)
                    direction, price = "sell", pair3_bids[0]
                elif where_is_transaction_coin_three[1] == 5:
                    coin_amount = round_value(coin_amount / float(pair3_asks[0]), pair=pair3)
                    direction, price = "buy", pair3_asks[0]
                pending_orders.append(f'{pair1} {direction} {coin_amount} {price}')
                
                # Removes orders for 0.0
                try: # prevents making an order with 0.0
                    for i in pending_orders:
                        if " 0.0 " in i:
                            raise
                except:
                    continue 

                # Actually execute the orders
                if len(pending_orders) >= 3: # Prevents orders of 3 or less pairs
                    #print("\n")
                    #print(f"\nFor pair: {pairs_list}\nFound an arb with a net of ${coin_amount-starting_amount_USD}")
                    #print(pending_orders)
                    os.system(f"echo '{pending_orders}' >> {os.getcwd()}/trades.pipe") # Sends all 3 orders in one go
            
if __name__ == "__main__":
    while True:
        find_tri_arb_path()
