// expect: error: Found:    String
// expect: Required: Int
// expect: 1 error found
import scala.language.implicitConversions

class A
class B:
  def answer: Int = 42
object A:
  given Conversion[A, B] with
    def apply(a: A): B = new B

trait Ops:
  extension (a: A) def answer: String = "extension"

def test(a: A)(using Ops): Int = a.answer
