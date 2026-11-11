// An extension of a given of an inner level against a conversion of an outer level: the nearer candidate is
// taken (dotty's Implicits.scala:1369-1372), the extension, whose result is no Int. At the same level the
// extension is taken too; a conversion imported in the same method stands at its parameters' level.
// expect: error: Found:    String
// expect: Required: Int
// expect: 3 errors found
import scala.language.implicitConversions

class A
class B:
  def answer: Int = 42
trait Ops:
  extension (a: A) def answer: String = "extension"
object Conversions:
  given Conversion[A, B] = _ => new B

def nearer(a: A)(using conversion: Conversion[A, B]): Int =
  def inner(using ops: Ops): Int = a.answer
  inner(using new Ops {})

def sameClause(a: A)(using ops: Ops, conversion: Conversion[A, B]): Int = a.answer

def importedInside(a: A)(using ops: Ops): Int =
  import Conversions.given
  a.answer
