package fix.wave6

// An extension whose first clause is a using clause the receiver's type depends on, called from a
// body of the jar and from source.
class W6Ctx:
  class Term(val s: String)
  def term(s: String): Term = new Term(s)

extension (using c: W6Ctx)(self: c.Term)
  def tagged[A](using ev: DummyImplicit): String = s"<${self.s}>"

object W6Use:
  def viaLib(using c: W6Ctx)(t: c.Term): String = t.tagged[Int]

// Members an object inherits from its trait, exported by name, renamed next to a wildcard.
trait W6Tags:
  final def main: String = "main tag"
  final def div: String = "div"
object W6Tags extends W6Tags

// An opaque type's variance, read from its lambda's parameter.
object W6Box:
  opaque type Box[+A] = List[A]
  def mk[A](a: A): Box[A] = List(a)
  extension [A](b: Box[A]) def items: List[A] = b

// A lambda of a body whose parameter is by-name, typed `(x: => Int) => ..` in the jar.
object W6Thunk:
  val twice: (=> Int) => Int = x => x + x
  def count(): String =
    var n = 0
    val r = twice({ n += 1; n })
    s"$r $n"
