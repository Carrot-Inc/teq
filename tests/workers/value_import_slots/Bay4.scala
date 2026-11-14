package shelves

object Bay4:
  def take0(): String =
    val sh4x0 = new Shelf("S4T0")
    import sh4x0.row.*
    label + doubled
  def take1(): String =
    val sh4x1 = new Shelf("S4T1")
    import sh4x1.row.*
    label + doubled
  def take2(): String =
    val sh4x2 = new Shelf("S4T2")
    import sh4x2.row.*
    label + doubled
  def take3(): String =
    val sh4x3 = new Shelf("S4T3")
    import sh4x3.row.*
    label + doubled
  def take4(): String =
    val sh4x4 = new Shelf("S4T4")
    import sh4x4.row.*
    label + doubled
  def take5(): String =
    val sh4x5 = new Shelf("S4T5")
    import sh4x5.row.*
    label + doubled
  def take6(): String =
    val sh4x6 = new Shelf("S4T6")
    import sh4x6.row.*
    label + doubled
  def take7(): String =
    val sh4x7 = new Shelf("S4T7")
    import sh4x7.row.*
    label + doubled
  def every(): String = take0() + "," + take1() + "," + take2() + "," + take3() + "," + take4() + "," + take5() + "," + take6() + "," + take7()
