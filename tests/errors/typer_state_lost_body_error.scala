// expect: 15:45: error: type mismatch: found String, required Int
// expect: 1 error found
// A body typed on demand for an inferred result inside an extension attempt that gives way to a
// conversion is its definition's: its error stands, as scalac reports it.
import scala.language.implicitConversions

class A
class B { def foo: Int = 42 }
given Conversion[A, B] = _ => B()

@main def run(): Unit =
  val n: Int = A().foo
  println(n)

extension (a: A) def foo = { val bad: Int = "s"; "ext" }
