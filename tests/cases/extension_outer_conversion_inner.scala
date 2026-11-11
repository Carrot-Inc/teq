import scala.language.implicitConversions

class A
class B:
  def answer: Int = 42
trait Ops:
  extension (a: A) def answer: String = "extension"

def test(a: A)(using ops: Ops): Int =
  def inner(using conversion: Conversion[A, B]): Int = a.answer
  inner(using new Conversion[A, B] { def apply(a: A): B = new B })

@main def run(): Unit = println(test(new A)(using new Ops {}))
