// A given's extension of an inner level against candidates that are no deeper givens: a conversion method of an
// outer block, which is a candidate of its own level (dotty's Implicits.scala:134-148, 316-325), and two
// conversions of the receiver's companion, which are searched only where the context has none
// (Implicits.scala:1786-1802) and whose ambiguity is not reached. The extension is taken; its result is no Int.
// expect: error: Found:    String
// expect: Required: Int
// expect: 2 errors found
import scala.language.implicitConversions

class A
class B:
  def answer: Int = 42
trait Ops:
  extension (a: A) def answer: String = "extension"

def method(a: A): Int =
  implicit def convert(x: A): B = new B
  def inner(using ops: Ops): Int = a.answer
  inner(using new Ops {})

class C
object C:
  given first: Conversion[C, B] = _ => new B
  given second: Conversion[C, B] = _ => new B
trait COps:
  extension (c: C) def answer: String = "extension"

def companions(c: C)(using ops: COps): Int = c.answer
