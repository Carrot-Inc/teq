package tills

object Desk6:
  def use0(): Int = Join.pair(second = Count.count(60), first = 0)
  def use1(): Int = Join.pair(second = Count.count(61), first = 1)
  def use2(): Int = Join.pair(second = Count.count(62), first = 2)
  def use3(): Int = Join.pair(second = Count.count(63), first = 3)
  def use4(): Int = Join.pair(second = Count.count(64), first = 4)
  def use5(): Int = Join.pair(second = Count.count(65), first = 5)
  def use6(): Int = Join.pair(second = Count.count(66), first = 6)
  def use7(): Int = Join.pair(second = Count.count(67), first = 7)
  def every(): Int = use0() + use1() + use2() + use3() + use4() + use5() + use6() + use7()
