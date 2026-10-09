// A right-associative extension selected on a qualifier, `q.op(a)`, is `op(q)(a)`: dotty's
// `extMethodApply` fills the first clause of the declaration's order (`Desugar.rightAssocParams`, the
// method's clause before the receiver's) with the qualifier, so the selection means what the infix
// `a op q` means, the qualifier evaluated first. Where no such extension takes the operands, the
// qualifier's conversion is tried (a String's `+:` through its sequence of characters), never the
// reading with the qualifier as the receiver; a std extension that stands for a member (`Array`'s
// `+:`) keeps the qualifier as its receiver. One of the receiver's implicit scope is taken alike.
// The qualifier is passed by name where the method's parameter is by-name, the argument is typed
// against the receiver's type (fixed by the qualifier for a generic one), a named argument may name
// the receiver, and an overload taking no list of its own does not take the argument list. The
// qualifier goes first where the overload taken has it by value, whatever another overload takes.
// Where the prefix `op(q)` does not take the qualifier, the conversion takes the arguments as
// written, neither typed against that candidate's receiver.
object Syntax:
  extension (n: Int)
    def +:(s: String): String = s + n
  extension (n: String)
    def +::(s: String)(tail: String): String = n + ":" + s + ":" + tail

object Shadowing:
  extension (s: String) def +:(n: Int): String = "never"

class C(val v: Int)
object C:
  extension (n: Int) def +:(c: C): C = C(c.v * 10 + n)

def q(s: String): String = { println("q " + s); s }
def a(n: Int): Int = { println("a " + n); n }

@main def run(): Unit =
  locally {
    import Syntax.*
    println(q("x").+:(a(2)))
    println("b".+::("a")("c"))
    println(q("y").+::(q("z"))(q("w")))
    println(2 +: "s")
    println(Array(1, 2).+:(0).toList)
    println(List(1, 2).+:(0))
  }
  locally {
    import Shadowing.*
    println("x".+:(2))
  }
  println(C(1).+:(2).v)
  println((3 +: C(4)).v)
  locally {
    import ByName.*
    println(side().+:(2))
  }
  locally {
    import Floaty.*
    println(2.+:(1.5))
  }
  locally {
    import Generic.*
    println(2.+:(x => x + 1))
  }
  locally {
    import Syntax.*
    println("x".+:(n = 2))
  }
  locally {
    import Mixed.*
    println("x".+:(2))
  }
  locally {
    import Lexical.*
    println((new D).+:(2))
  }
  more()

object ByName:
  extension (n: Int) def +:(s: => String): String = s + s + n
var count = 0
def side(): String = { count += 1; count.toString }

object Floaty:
  extension (n: Float) def +:(s: Int): Float = n
object Generic:
  extension [A](f: A => A) def +:(x: A): A = f(x)

object Mixed:
  extension (n: Int) def +:(s: String): String = s + n
  extension (s: String) def +: : String = "nullary"

class D
object D:
  extension (n: Int) def +:(d: D): String = "companion"
object Lexical:
  extension (n: String) def +: : String = "lexical"

object Order:
  extension (n: Int) def +:(s: String): String = s + n
  extension (n: Boolean) def +:(s: => String): String = s
var trace = ""
def tq(): String = { trace += "q"; "x" }
def ta(): Int = { trace += "a"; 2 }

object FloatOnly:
  extension (n: Float) def +:(s: Int): Float = n

import scala.language.implicitConversions
class R
class V:
  def +:(f: Int => Int): Int = f(2)
given Conversion[R, V] = _ => new V
object StringOnly:
  extension (s: String) def +:(n: Int): String = s

def more(): Unit =
  locally {
    import Order.*
    println(tq().+:(ta()))
    println(trace)
  }
  locally {
    import FloatOnly.*
    val xs = "x".+:(1.1)
    println((xs.head: Any) == (1.1: Any))
  }
  locally {
    import StringOnly.*
    println((new R).+:(x => x + 1))
  }
