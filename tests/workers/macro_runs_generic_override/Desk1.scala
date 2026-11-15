package tills

object Desk1:
  def use0(): Int = Join.pair(second = Count.count(10), first = 0)
  def use1(): Int = Join.pair(second = Count.count(11), first = 1)
  def use2(): Int = Join.pair(second = Count.count(12), first = 2)
  def use3(): Int = Join.pair(second = Count.count(13), first = 3)
  def use4(): Int = Join.pair(second = Count.count(14), first = 4)
  def use5(): Int = Join.pair(second = Count.count(15), first = 5)
  def use6(): Int = Join.pair(second = Count.count(16), first = 6)
  def use7(): Int = Join.pair(second = Count.count(17), first = 7)
  def every(): Int = use0() + use1() + use2() + use3() + use4() + use5() + use6() + use7()
