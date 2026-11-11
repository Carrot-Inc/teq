object Words:
  extension (value: String) def doubled: String = value + value
object Counts:
  extension (value: Int) def doubled: Int = value * 2
export Words.doubled
export Counts.doubled

@main def run(): Unit =
  println("ab".doubled)
  println(2.doubled)
