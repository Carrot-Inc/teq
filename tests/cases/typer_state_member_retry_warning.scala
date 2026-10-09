// A member whose application fails at its second argument gives way to an extension, which reuses
// the first argument's typing, `{ 1; 2 }`: its "pure expression" warning went with the member's
// application, as scalac's cached typing keeps nothing of the failed state's reporter (scalac `5`,
// no warning).
class Box:
  def put(y: Int, z: String): Int = 0

extension (b: Box) def put(y: Int, z: Int): Int = y + z

@main def run(): Unit =
  println(Box().put({ 1; 2 }, 3))
