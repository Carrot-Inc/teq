package vaults

object Desk7:
  def use0(): Int = Join.pair(second = Tally.tally(70), first = 0)
  def use1(): Int = Join.pair(second = Tally.tally(71), first = 1)
  def use2(): Int = Join.pair(second = Tally.tally(72), first = 2)
  def use3(): Int = Join.pair(second = Tally.tally(73), first = 3)
  def use4(): Int = Join.pair(second = Tally.tally(74), first = 4)
  def use5(): Int = Join.pair(second = Tally.tally(75), first = 5)
  def use6(): Int = Join.pair(second = Tally.tally(76), first = 6)
  def use7(): Int = Join.pair(second = Tally.tally(77), first = 7)
  def every(): Int = use0() + use1() + use2() + use3() + use4() + use5() + use6() + use7()
