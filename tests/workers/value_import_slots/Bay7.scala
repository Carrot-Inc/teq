package shelves

object Bay7:
  def take0(): String =
    val sh7x0 = new Shelf("S7T0")
    import sh7x0.row.*
    label + doubled
  def take1(): String =
    val sh7x1 = new Shelf("S7T1")
    import sh7x1.row.*
    label + doubled
  def take2(): String =
    val sh7x2 = new Shelf("S7T2")
    import sh7x2.row.*
    label + doubled
  def take3(): String =
    val sh7x3 = new Shelf("S7T3")
    import sh7x3.row.*
    label + doubled
  def take4(): String =
    val sh7x4 = new Shelf("S7T4")
    import sh7x4.row.*
    label + doubled
  def take5(): String =
    val sh7x5 = new Shelf("S7T5")
    import sh7x5.row.*
    label + doubled
  def take6(): String =
    val sh7x6 = new Shelf("S7T6")
    import sh7x6.row.*
    label + doubled
  def take7(): String =
    val sh7x7 = new Shelf("S7T7")
    import sh7x7.row.*
    label + doubled
  def every(): String = take0() + "," + take1() + "," + take2() + "," + take3() + "," + take4() + "," + take5() + "," + take6() + "," + take7()
