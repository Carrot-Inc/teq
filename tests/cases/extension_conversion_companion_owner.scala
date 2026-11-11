// A conversion and a given's extension imported at one level: the given whose object's class derives from the
// other object's class is taken (dotty's Applications.compareOwner over the companion classes of two modules).
import scala.language.implicitConversions

class A
class B:
  def answer: Int = 42

trait Ops:
  extension (a: A) def answer: String = "extension"

class Parent
object Parent:
  given ops: Ops = new Ops {}

class Child extends Parent
object Child:
  given convert: Conversion[A, B] = _ => new B

def result(a: A): Int =
  import Parent.given
  import Child.given
  a.answer

@main def run(): Unit = println(result(new A))
