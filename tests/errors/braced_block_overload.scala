// expect: 15:24: error: Ambiguous overload
// expect: 18:7: error: Ambiguous overload
// A block in braces stays a block, of one expression too: a selection on it chooses nothing among the overloads
// of the method called inside it, where the block is a right side and where it is alone the indented body of a
// case. scalac: "Ambiguous overload. The overloaded alternatives of method f in object O .. both match expected
// type <?>" at each call.
trait T
trait U
object E { given T = new T {}; given U = new U {} }
import E.given
class C { def sel: String = "selected" }
object O:
  def f(using T): C = new C
  def f(using U): Int = 0
def result: String = { O.f }.sel
def caseBody(x: Int): String = x match {
  case 1 =>
    { O.f }
  .sel
  case _ => "other"
}
