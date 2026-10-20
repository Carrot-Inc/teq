// expect: 16:11: error: member refused
// expect: 17:11: error: member refused
// expect: 2 errors found
// absent: extension
// A plain inline member that applies to its first argument list keeps the error its expansion
// reports, past that boundary, where an extension of the name would take the arguments (scalac's
// `tryInsertImplicitOnQualifier`): the call is the member's, its `error` the inlining phase's, the
// program typed without an error (member_fallback_boundary has the boundary's other errors).
import scala.compiletime.error
class B { inline def f(x: Int): String = error("member refused") }
extension (b: B) def f(x: Any): String = "extension"
class Box[T]
class J { inline def f[T](x: Box[T]): String = error("member refused") }
extension (j: J) def f(x: Box[Int]): String = "extension"
@main def run(): Unit =
  println(B().f(1))
  println(J().f(new Box[Int]))
