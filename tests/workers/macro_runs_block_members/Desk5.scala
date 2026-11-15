package vaults

object Desk5:
  def use0(): Int = Join.pair(second = Tally.tally(50), first = 0)
  def use1(): Int = Join.pair(second = Tally.tally(51), first = 1)
  def use2(): Int = Join.pair(second = Tally.tally(52), first = 2)
  def use3(): Int = Join.pair(second = Tally.tally(53), first = 3)
  def use4(): Int = Join.pair(second = Tally.tally(54), first = 4)
  def use5(): Int = Join.pair(second = Tally.tally(55), first = 5)
  def use6(): Int = Join.pair(second = Tally.tally(56), first = 6)
  def use7(): Int = Join.pair(second = Tally.tally(57), first = 7)
  def every(): Int = use0() + use1() + use2() + use3() + use4() + use5() + use6() + use7()
