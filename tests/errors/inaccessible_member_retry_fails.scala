class Guarded:
  protected def stamp: String = "member"
  protected def mark(n: Int): String = s"member$n"
  protected val seal: String = "member"
  protected def tag(n: Int): String = s"member$n"
  private def inner: String = "member"
  private def depth: String = "member"
class Evidence
extension (g: Guarded)(using Evidence) def stamp: String = "extension"
extension (g: Guarded)(using Evidence) def tag(n: Int): String = "extension"
extension [T <: String](t: T) def seal: String = "extension"
extension (g: Guarded)(using Evidence) def inner: String = "extension"
object Left { extension (g: Guarded) def mark(n: Int): String = "left"; extension (g: Guarded) def depth: String = "left" }
object Right { extension (g: Guarded) def mark(n: Int): String = "right"; extension (g: Guarded) def depth: String = "right" }
import Left.*
import Right.*
def uses(g: Guarded): Unit =
  println(g.stamp)
  println(g.mark(1))
  println(g.seal)
  println(g.inner)
  println(g.depth)
  println(g.tag(absent))
// A member the site may not access is left to an extension method of its name; where that extension is not taken
// for the receiver (its using clause has no given, two imports bring one each, its type parameter's bound is not
// met) the attempt is taken back and the access is what is told, as scalac tells it: nothing of the extension, and
// an argument's own error beside it.
// expect: 18:11: error: method stamp in class Guarded cannot be accessed as a member of Guarded from here
// expect: 19:11: error: method mark in class Guarded cannot be accessed as a member of Guarded from here
// expect: 20:11: error: value seal in class Guarded cannot be accessed as a member of Guarded from here
// expect: 21:11: error: inner is private to Guarded
// expect: 22:11: error: depth is private to Guarded
// expect: 23:11: error: method tag in class Guarded cannot be accessed as a member of Guarded from here
// expect: 23:17: error: not found: absent
// expect: 7 errors found
// absent: no given instance
// absent: ambiguous extension methods
// absent: is not applicable
