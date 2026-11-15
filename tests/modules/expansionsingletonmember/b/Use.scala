package vatuse

import vat.*

object Shed:
  val made = new Rod("b" + 1)

def read: Int = Gauge.make("abc") + Shed.made.len
