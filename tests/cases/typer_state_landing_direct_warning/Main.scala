// A member's argument that is a macro's expansion itself is reused by the extension the retry
// applies, with its warning, as scalac's retry expands it again: the warning and 5.
class Box:
  def put(y: Int, z: String): Int = 0
extension (b: Box) def put(y: Int, z: Int): Int = y + z
@main def run(): Unit = println(Box().put(M.next, 3))
