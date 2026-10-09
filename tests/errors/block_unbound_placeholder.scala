// expect: 7:27: error: unbound placeholder parameter; incorrect use of `_`
// expect: 8:30: error: unbound placeholder parameter; incorrect use of `_`
// A placeholder that is a block's statement alone binds it to no function, whatever continues
// the block (scalac's `checkNoEscapingPlaceholders` around a block's statements).
object Main:
  def main(args: Array[String]): Unit =
    val f: Int => Int = { _ } + 10
    println(List(1, 2).map({ _ } + 10))
