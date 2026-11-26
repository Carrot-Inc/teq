// A named tuple as the left operand of a right-associative call is an argument of the receiver's type.
extension (x: Int)
  def %:(y: String): String = y + x
@main def run(): Unit = println((x = 7) %: "r")
// expect: 4:33: error: Found:    (x : Int)
