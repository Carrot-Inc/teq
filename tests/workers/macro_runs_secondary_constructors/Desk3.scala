package crates

object Desk3:
  def use0(): String = Join.pair(second = Probe.probe(30), first = 0)
  def use1(): String = Join.pair(second = Probe.probe(31), first = 1)
  def use2(): String = Join.pair(second = Probe.probe(32), first = 2)
  def use3(): String = Join.pair(second = Probe.probe(33), first = 3)
  def use4(): String = Join.pair(second = Probe.probe(34), first = 4)
  def use5(): String = Join.pair(second = Probe.probe(35), first = 5)
  def use6(): String = Join.pair(second = Probe.probe(36), first = 6)
  def use7(): String = Join.pair(second = Probe.probe(37), first = 7)
  def every(): String = use0() + " " + use1() + " " + use2() + " " + use3() + " " + use4() + " " + use5() + " " + use6() + " " + use7()
