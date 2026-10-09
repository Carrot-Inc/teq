// An argument that is a macro's expansion in parentheses, which are no tree for scalac either, or
// a later argument, is reused by the extension the retry applies with its warning, as scalac's
// retry expands it again: the warnings, 5 and 6. One inside braces, an ascription or another
// inline method's argument declines.
class Box:
  def put(y: Int, z: String): Int = 0
  def put2(a: Int, y: Int, z: String): Int = 0
extension (b: Box) def put(y: Int, z: Int): Int = y + z
extension (b: Box) def put2(a: Int, y: Int, z: Int): Int = a + y + z
@main def run(): Unit =
  println(Box().put((M.next), 3))
  println(Box().put2(1, M.next, 3))
