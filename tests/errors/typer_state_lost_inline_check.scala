// expect: 15:60: error: type mismatch: found String, required Int
// expect: 1 error found
// An inline definition checked on demand inside an extension attempt that gives way to a
// conversion: the check's error stands, as scalac reports it.
import scala.language.implicitConversions

class A
class B { def foo: Int = 42 }
given Conversion[A, B] = _ => B()

@main def run(): Unit =
  val n: Int = A().foo
  println(n)

extension (a: A) inline def foo: String = { val bad: Int = "s"; "ext" }
