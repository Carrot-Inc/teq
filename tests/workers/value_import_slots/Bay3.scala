package shelves

object Bay3:
  def take0(): String =
    val sh3x0 = new Shelf("S3T0")
    import sh3x0.row.*
    label + doubled
  def take1(): String =
    val sh3x1 = new Shelf("S3T1")
    import sh3x1.row.*
    label + doubled
  def take2(): String =
    val sh3x2 = new Shelf("S3T2")
    import sh3x2.row.*
    label + doubled
  def take3(): String =
    val sh3x3 = new Shelf("S3T3")
    import sh3x3.row.*
    label + doubled
  def take4(): String =
    val sh3x4 = new Shelf("S3T4")
    import sh3x4.row.*
    label + doubled
  def take5(): String =
    val sh3x5 = new Shelf("S3T5")
    import sh3x5.row.*
    label + doubled
  def take6(): String =
    val sh3x6 = new Shelf("S3T6")
    import sh3x6.row.*
    label + doubled
  def take7(): String =
    val sh3x7 = new Shelf("S3T7")
    import sh3x7.row.*
    label + doubled
  def every(): String = take0() + "," + take1() + "," + take2() + "," + take3() + "," + take4() + "," + take5() + "," + take6() + "," + take7()
