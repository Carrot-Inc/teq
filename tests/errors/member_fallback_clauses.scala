// expect: 25:19: error: type mismatch: found String, required Int
// expect: 26:19: error: type mismatch: found String, required Int
// expect: 26:24: error: not found: undefinedName
// expect: 27:22: error: type mismatch: found Int, required String
// expect: 29:24: error: type mismatch: found Int, required String
// expect: 32:30: error: type mismatch: found String, required Int
// expect: 33:19: error: type mismatch: found String, required Int
// A curried member call is applied clause by clause, the innermost first, as dotty's
// `realApply` types it (Applications.scala 1315): where the first plain clause fails, the
// later clauses are typed alone, once, for their own errors (1438: `1` reports nothing against
// `b: String`, `undefinedName` is not found); where it fits, a later clause's mismatch is the
// error. The retry on the qualifier is the innermost application's: an extension that takes the
// first clause and fails in a later one is the application (its error at `D().f("s")(1)`, not the member's),
// and a member whose first clause is a using one, given or inferred, is applied to that clause
// first, an application `tryInsertImplicitOnQualifier` retries nothing on (Typer.scala 4245).
class C { def f(a: Int)(b: String): String = b }
class D { def f(a: Int)(b: String): String = "member" }
extension (d: D) def f(a: String)(b: String): String = b
class X
given X = X()
class E { def f(using x: X)(a: Int): String = "member" }
extension (e: E) def f(using x: X)(a: String): String = "extension"
object Main:
  def main(args: Array[String]): Unit =
    println(C().f("s")(1))
    println(C().f("s")(undefinedName))
    println(C().f(1)(2))
    println(D().f("s")("b"))
    println(D().f("s")(1))
    println(E().f(using X())(1))
    println(E().f(1))
    println(E().f(using X())("s"))
    println(E().f("s"))
