// A member call whose first clause the member cannot take gives way to an extension, and an
// argument of a later clause is typed once: dotty types the innermost application first and
// the clauses after it once that settles (`realApply`, Applications.scala 1315), so the macro
// in a later clause expands once whatever the retry's outcome. One, two and three clauses, a
// using clause, a by-name later argument, an inline given a using clause resolves (expanded once:
// the lexical extension is applied once, whether alone or beside a companion's or a given's
// extension of the name); the counter after each shows the expansions so far.

class X
given X = X()

class C1 { def f(a: Int): String = "member" }
extension (c: C1) def f(a: String): String = "one " + a
class C2 { def f(a: Int)(b: String): String = "member" }
extension (c: C2) def f(a: String)(b: String): String = "two " + a + b
class C3 { def f(a: Int)(b: String)(d: String): String = "member" }
extension (c: C3) def f(a: String)(b: String)(d: String): String = "three " + a + b + d
class Cu { def f(a: Int)(using b: String): String = "member" }
extension (c: Cu) def f(a: String)(using b: String): String = "using " + a + b
class Cb { def f(a: Int)(b: => String): String = "member" }
extension (c: Cb) def f(a: String)(b: => String): String = "byname " + a + b + b
inline given str: String = next
class Cg { def f(a: Int)(using b: String): String = "member" }
extension (c: Cg) def f(a: String)(using b: String): String = "given " + a + b
class Cc { def f(a: Int)(using b: String): String = "member" }
object Cc:
  extension (c: Cc) def f(a: Boolean)(using b: String): String = "companion " + b
extension (c: Cc) def f(a: String)(using b: String): String = "lexical " + a + b
trait Ops:
  extension (c: Cv) def f(a: Boolean)(using b: String): String = "via given " + b
given Ops = new Ops {}
class Cv { def f(a: Int)(using b: String): String = "member" }
extension (c: Cv) def f(a: String)(using b: String): String = "lexical over given " + a + b

@main def run(): Unit =
  println(C1().f(next))
  println(next)
  println(C2().f("s")(next))
  println(next)
  println(C3().f("s")(next)(next))
  println(next)
  println(Cu().f("s")(using next))
  println(next)
  println(Cb().f("s")(next))
  println(next)
  println(Cg().f("s"))
  println(next)
  println(Cc().f("s"))
  println(next)
  println(Cv().f("s"))
  println(next)
