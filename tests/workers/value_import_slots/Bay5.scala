package shelves

object Bay5:
  def take0(): String =
    val sh5x0 = new Shelf("S5T0")
    import sh5x0.row.*
    label + doubled
  def take1(): String =
    val sh5x1 = new Shelf("S5T1")
    import sh5x1.row.*
    label + doubled
  def take2(): String =
    val sh5x2 = new Shelf("S5T2")
    import sh5x2.row.*
    label + doubled
  def take3(): String =
    val sh5x3 = new Shelf("S5T3")
    import sh5x3.row.*
    label + doubled
  def take4(): String =
    val sh5x4 = new Shelf("S5T4")
    import sh5x4.row.*
    label + doubled
  def take5(): String =
    val sh5x5 = new Shelf("S5T5")
    import sh5x5.row.*
    label + doubled
  def take6(): String =
    val sh5x6 = new Shelf("S5T6")
    import sh5x6.row.*
    label + doubled
  def take7(): String =
    val sh5x7 = new Shelf("S5T7")
    import sh5x7.row.*
    label + doubled
  def every(): String = take0() + "," + take1() + "," + take2() + "," + take3() + "," + take4() + "," + take5() + "," + take6() + "," + take7()
