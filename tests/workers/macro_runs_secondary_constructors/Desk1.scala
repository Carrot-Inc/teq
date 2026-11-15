package crates

object Desk1:
  def use0(): String = Join.pair(second = Probe.probe(10), first = 0)
  def use1(): String = Join.pair(second = Probe.probe(11), first = 1)
  def use2(): String = Join.pair(second = Probe.probe(12), first = 2)
  def use3(): String = Join.pair(second = Probe.probe(13), first = 3)
  def use4(): String = Join.pair(second = Probe.probe(14), first = 4)
  def use5(): String = Join.pair(second = Probe.probe(15), first = 5)
  def use6(): String = Join.pair(second = Probe.probe(16), first = 6)
  def use7(): String = Join.pair(second = Probe.probe(17), first = 7)
  def every(): String = use0() + " " + use1() + " " + use2() + " " + use3() + " " + use4() + " " + use5() + " " + use6() + " " + use7()
