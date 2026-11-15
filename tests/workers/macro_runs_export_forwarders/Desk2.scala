package relays

object Desk2:
  def use0(): String = Join.pair(second = Probe.probe(20), first = 0)
  def use1(): String = Join.pair(second = Probe.probe(21), first = 1)
  def use2(): String = Join.pair(second = Probe.probe(22), first = 2)
  def use3(): String = Join.pair(second = Probe.probe(23), first = 3)
  def use4(): String = Join.pair(second = Probe.probe(24), first = 4)
  def use5(): String = Join.pair(second = Probe.probe(25), first = 5)
  def use6(): String = Join.pair(second = Probe.probe(26), first = 6)
  def use7(): String = Join.pair(second = Probe.probe(27), first = 7)
  def every(): String = use0() + " " + use1() + " " + use2() + " " + use3() + " " + use4() + " " + use5() + " " + use6() + " " + use7()
