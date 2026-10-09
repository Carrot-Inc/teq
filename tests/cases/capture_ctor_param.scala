// A constructor's parameter is a binding of the constructor's JavaScript function, whose name is
// the parameter's field: one named like a global the constructor's text reads (`Math` of the
// multiplication of two `Int`s) is named apart there, and the field keeps its name.
class Stored(Math: Int):
  val y = Math * 2
  def z = Math * 3

class Passed(Math: Int):
  val y = Math * 4

@main def main(): Unit =
  val s = Stored(4)
  println(s.y)
  println(s.z)
  println(Passed(5).y)
