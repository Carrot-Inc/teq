package forgeuse

import forge.*

def read: String = Gauge.make("abc") + "/" + Shed.made.label
