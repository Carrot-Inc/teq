package shelves

object Bay1:
  def take0(): String =
    val sh1x0 = new Shelf("S1T0")
    import sh1x0.row.*
    label + doubled
  def take1(): String =
    val sh1x1 = new Shelf("S1T1")
    import sh1x1.row.*
    label + doubled
  def take2(): String =
    val sh1x2 = new Shelf("S1T2")
    import sh1x2.row.*
    label + doubled
  def take3(): String =
    val sh1x3 = new Shelf("S1T3")
    import sh1x3.row.*
    label + doubled
  def take4(): String =
    val sh1x4 = new Shelf("S1T4")
    import sh1x4.row.*
    label + doubled
  def take5(): String =
    val sh1x5 = new Shelf("S1T5")
    import sh1x5.row.*
    label + doubled
  def take6(): String =
    val sh1x6 = new Shelf("S1T6")
    import sh1x6.row.*
    label + doubled
  def take7(): String =
    val sh1x7 = new Shelf("S1T7")
    import sh1x7.row.*
    label + doubled
  def every(): String = take0() + "," + take1() + "," + take2() + "," + take3() + "," + take4() + "," + take5() + "," + take6() + "," + take7()
