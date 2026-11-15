package kilnuse

import kiln.*

def read: Int = Gauge.make("abc") + Gauge.make("de") + Shed.made.tag
