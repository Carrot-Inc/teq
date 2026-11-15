package milluse

import mill.*

def read: String = Gauge.make("abc") + "/" + Shed.made.label
