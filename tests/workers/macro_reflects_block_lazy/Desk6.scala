package hearths

object Desk6:
  def use0(): String = Join.pair(second = Probe.probe(48), first = 0)
  def use1(): String = Join.pair(second = Probe.probe(49), first = 1)
  def use2(): String = Join.pair(second = Probe.probe(50), first = 2)
  def use3(): String = Join.pair(second = Probe.probe(51), first = 3)
  def use4(): String = Join.pair(second = Probe.probe(52), first = 4)
  def use5(): String = Join.pair(second = Probe.probe(53), first = 5)
  def use6(): String = Join.pair(second = Probe.probe(54), first = 6)
  def use7(): String = Join.pair(second = Probe.probe(55), first = 7)
  def every(): String = List(use0(), use1(), use2(), use3(), use4(), use5(), use6(), use7()).mkString(" ")
