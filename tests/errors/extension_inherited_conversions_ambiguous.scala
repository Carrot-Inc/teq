// A class's own conversions against an extension of a given it inherits: candidates of one level, where the
// ones the derived class owns come first (dotty's Applications.compareOwner), so their ambiguity is the answer
// and the inherited given's extension is not taken.
// expect: error: Found:    (a : A)
// expect: Required: ?{ answer: ? }
// expect: both given instance first in class Child and given instance second in class Child provide an extension method `answer` on (a : A)
// expect: 1 error found
import scala.language.implicitConversions

class A
class B:
  def answer: Int = 42
trait Ops:
  extension (a: A) def answer: String = "extension"
class Parent:
  given ops: Ops = new Ops {}
class Child extends Parent:
  given first: Conversion[A, B] = _ => new B
  given second: Conversion[A, B] = _ => new B
  def result(a: A): Int = a.answer
