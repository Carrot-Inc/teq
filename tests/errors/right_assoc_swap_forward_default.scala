// The default of a right-associative extension's receiver cannot read the method's own parameter.
object Lib:
  extension (x: Int = y)
    def %:(y: Int): Int = x + y
@main def run(): Unit = println("ok")
// expect: 3:23: error: right-associative extension method cannot have a forward reference to y
