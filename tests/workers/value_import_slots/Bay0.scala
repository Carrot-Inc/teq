package shelves

object Bay0:
  def take0(): String =
    val sh0x0 = new Shelf("S0T0")
    import sh0x0.row.*
    label + doubled
  def take1(): String =
    val sh0x1 = new Shelf("S0T1")
    import sh0x1.row.*
    label + doubled
  def take2(): String =
    val sh0x2 = new Shelf("S0T2")
    import sh0x2.row.*
    label + doubled
  def take3(): String =
    val sh0x3 = new Shelf("S0T3")
    import sh0x3.row.*
    label + doubled
  def take4(): String =
    val sh0x4 = new Shelf("S0T4")
    import sh0x4.row.*
    label + doubled
  def take5(): String =
    val sh0x5 = new Shelf("S0T5")
    import sh0x5.row.*
    label + doubled
  def take6(): String =
    val sh0x6 = new Shelf("S0T6")
    import sh0x6.row.*
    label + doubled
  def take7(): String =
    val sh0x7 = new Shelf("S0T7")
    import sh0x7.row.*
    label + doubled
  def every(): String = take0() + "," + take1() + "," + take2() + "," + take3() + "," + take4() + "," + take5() + "," + take6() + "," + take7()
