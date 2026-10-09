// A right-associative extension method called as a method takes its argument lists in the
// order of its declaration (dotty's `Desugar.extMethod`, `rightAssocParams`): the method's first
// clause, then the receiver's with the extension's using clauses after it, then the method's
// others. A using clause first in the method keeps the receiver's clause first, and an empty
// first clause is no operator's. The arguments are evaluated as written, after the receiver the
// method is selected on (`Applications.liftFun`), a named one too, a by-name one where it is used.
// Infix calls and ordinary curried methods named alike are unaffected.
object Syntax:
  extension (n: Int)
    def +:(s: String): String = s + n
  extension (n: String)
    def +::(s: String)(tail: String): String = n + ":" + s + ":" + tail
  extension [A](n: A)(using o: Ordering[A])
    def ++:[B](s: B)(t: Int): String = s"$s:$n:$t:${o.compare(n, n)}"
  extension (n: Int)
    def -:(using s: String)(y: Int): String = s + n + y
  def *:(n: Int)(s: String): String = s * n
  extension (n: Int)
    def /:(): Int = n
  extension (n: String)
    def %:(s: => String)(t: String): String = { println("body"); n + ":" + s + ":" + t }

def mark(s: String): String = { println(s); s }

class Instance:
  extension (n: String)
    def +:(s: String): String = n + ":" + s
def instance: Instance = { println("receiver"); new Instance }

@main def run(): Unit =
  println(Syntax.+:("x")(2))
  println(Syntax.+::("a")("b")("c"))
  println(Syntax.++:("s")(5)(7))
  println(Syntax.-:(1)(using "s")(2))
  println(Syntax.*:(2)("ab"))
  println(Syntax.+::(mark("a"))(mark("b"))(mark("c")))
  println(Syntax.%:(mark("d"))(mark("e"))(mark("f")))
  println(Syntax./:(4)())
  println(instance.+:(mark("g"))(mark("h")))
  println(Syntax.+::(s = mark("i"))(mark("j"))(mark("k")))
  println(Syntax.%:(s = mark("l"))(mark("m"))(mark("n")))
  import Syntax.*
  println(2 +: "x")
  println(("a" +:: "b")("c"))
  println(("q" ++: 6)(8))
