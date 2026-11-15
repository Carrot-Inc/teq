package hearths

object Desk7:
  def use0(): String = Join.pair(second = Probe.probe(56), first = 0)
  def use1(): String = Join.pair(second = Probe.probe(57), first = 1)
  def use2(): String = Join.pair(second = Probe.probe(58), first = 2)
  def use3(): String = Join.pair(second = Probe.probe(59), first = 3)
  def use4(): String = Join.pair(second = Probe.probe(60), first = 4)
  def use5(): String = Join.pair(second = Probe.probe(61), first = 5)
  def use6(): String = Join.pair(second = Probe.probe(62), first = 6)
  def use7(): String = Join.pair(second = Probe.probe(63), first = 7)
  def every(): String = List(use0(), use1(), use2(), use3(), use4(), use5(), use6(), use7()).mkString(" ")
