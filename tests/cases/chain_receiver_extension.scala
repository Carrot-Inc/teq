// An extension whose receiver is a `*:` chain, written or behind an alias, applies to a tuple, which conforms to the
// chain by the subtype check's rule, not by its base classes (master: "value first is not a member of (Int, String)"
// on JavaScript and in the interpreter, whose tuple classes have no `*:` among their bases).
object Ext:
  type C = Int *: String *: EmptyTuple
  extension (x: Int *: String *: EmptyTuple) def first: Int = x._1
  extension (x: C) def second: String = x._2
import Ext.*
@main def run(): Unit =
  println((6, "e").first)
  val t: (Int, String) = (7, "f")
  println(t.first + t.second)
