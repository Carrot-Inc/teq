package crates

object Desk6:
  def use0(): String = Join.pair(second = Probe.probe(60), first = 0)
  def use1(): String = Join.pair(second = Probe.probe(61), first = 1)
  def use2(): String = Join.pair(second = Probe.probe(62), first = 2)
  def use3(): String = Join.pair(second = Probe.probe(63), first = 3)
  def use4(): String = Join.pair(second = Probe.probe(64), first = 4)
  def use5(): String = Join.pair(second = Probe.probe(65), first = 5)
  def use6(): String = Join.pair(second = Probe.probe(66), first = 6)
  def use7(): String = Join.pair(second = Probe.probe(67), first = 7)
  def every(): String = use0() + " " + use1() + " " + use2() + " " + use3() + " " + use4() + " " + use5() + " " + use6() + " " + use7()
