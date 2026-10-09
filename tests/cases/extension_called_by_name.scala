// An extension method is a term of its scope like any method (dotty's `findRef`): named by an
// application's head, through an import, in its own object or class, or through an imported value,
// it is applied as its selection on that receiver is, its first argument list the receiver's, a
// right-associative one's lists in its declaration's order. It takes the binding precedence of
// its scope: an inner import before an outer one, a definition of the enclosing scope before an
// import of an outer one, the named and the wildcard imports as for any name.
object Syntax:
  extension (n: String)
    def +:(s: String): String = n + ":" + s
    def tag: String = "tag:" + n
    def ++++(s: String): String = n + s
  def inside: String = tag("in")
  def inside2: String = +:("a")("b")

class K:
  extension (n: Int) def twice: Int = n * 2
  def use: Int = twice(4)

object A:
  extension (n: Int) def mark: String = "a-ext"
object B:
  def mark(n: Int): String = "b-def"
object C:
  extension (s: String) def mark: String = "c-ext"

object Ops:
  extension (n: Int) def pair(m: Int): String = s"$n-$m"
  extension (n: Int) def show: String = s"int $n"
  extension (s: String) def show: String = s"string $s"

class Holder:
  extension (n: Int) def scaled: Int = n * 10
val holder = Holder()

object User:
  import A.*
  def mark(n: Int): String = "user-def"
  def use: String = mark(1)

import Syntax.*
import B.*

@main def run(): Unit =
  println(tag("x"))
  println(++++("a")("b"))
  println(+:("a")("b"))
  println(Syntax.inside)
  println(Syntax.inside2)
  println(K().use)
  locally {
    import A.*
    println(mark(1))
    locally {
      import C.*
      println(mark("s"))
    }
  }
  println(mark(2))
  println(User.use)
  locally {
    import Ops.*
    val f = pair(1)
    println(f(2))
    println(show(3))
    println(show("s"))
  }
  locally {
    import holder.*
    println(scaled(3))
  }
  locally {
    import Renaming.{tag as renamed}
    println(renamed(2))
  }

object Renaming:
  extension (x: Int) def tag: Int = x + 1
  def renamed(x: Int): Int = 99
