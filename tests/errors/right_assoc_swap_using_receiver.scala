// A using clause cannot stand for an extension's receiver.
object Lib:
  extension (using x: String)
    def %:(y: Int): String = x + y
@main def run(): Unit =
  println(Lib.%:(7)(using "x"))
// expect: 4:1: error: '(' expected, but 'end of statement' found
// expect: 6:17: error: No given instance of type String was found for parameter x of method %: in object Lib
