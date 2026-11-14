package shelves

object Bay2:
  def take0(): String =
    val sh2x0 = new Shelf("S2T0")
    import sh2x0.row.*
    label + doubled
  def take1(): String =
    val sh2x1 = new Shelf("S2T1")
    import sh2x1.row.*
    label + doubled
  def take2(): String =
    val sh2x2 = new Shelf("S2T2")
    import sh2x2.row.*
    label + doubled
  def take3(): String =
    val sh2x3 = new Shelf("S2T3")
    import sh2x3.row.*
    label + doubled
  def take4(): String =
    val sh2x4 = new Shelf("S2T4")
    import sh2x4.row.*
    label + doubled
  def take5(): String =
    val sh2x5 = new Shelf("S2T5")
    import sh2x5.row.*
    label + doubled
  def take6(): String =
    val sh2x6 = new Shelf("S2T6")
    import sh2x6.row.*
    label + doubled
  def take7(): String =
    val sh2x7 = new Shelf("S2T7")
    import sh2x7.row.*
    label + doubled
  def every(): String = take0() + "," + take1() + "," + take2() + "," + take3() + "," + take4() + "," + take5() + "," + take6() + "," + take7()
