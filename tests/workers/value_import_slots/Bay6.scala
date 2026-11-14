package shelves

object Bay6:
  def take0(): String =
    val sh6x0 = new Shelf("S6T0")
    import sh6x0.row.*
    label + doubled
  def take1(): String =
    val sh6x1 = new Shelf("S6T1")
    import sh6x1.row.*
    label + doubled
  def take2(): String =
    val sh6x2 = new Shelf("S6T2")
    import sh6x2.row.*
    label + doubled
  def take3(): String =
    val sh6x3 = new Shelf("S6T3")
    import sh6x3.row.*
    label + doubled
  def take4(): String =
    val sh6x4 = new Shelf("S6T4")
    import sh6x4.row.*
    label + doubled
  def take5(): String =
    val sh6x5 = new Shelf("S6T5")
    import sh6x5.row.*
    label + doubled
  def take6(): String =
    val sh6x6 = new Shelf("S6T6")
    import sh6x6.row.*
    label + doubled
  def take7(): String =
    val sh6x7 = new Shelf("S6T7")
    import sh6x7.row.*
    label + doubled
  def every(): String = take0() + "," + take1() + "," + take2() + "," + take3() + "," + take4() + "," + take5() + "," + take6() + "," + take7()
