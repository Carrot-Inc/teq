// jars: scala-library traitfields-lib
// A class mixing in a jar trait with parameters holds them as scalac's `Mixin` makes them (`traitInits`): each a
// field set before the trait's `$init$`, its getter the trait's abstract accessor (a private parameter's and a using
// parameter's expanded, an anonymous one's `x$1`), a var's setter, a default from the trait's companion, a trait
// of parameters alone initialised by its fields (it has no `$init$`). master refused trait parameters on the JVM.
import tfl.*

def tag(s: String): String = { println("tag " + s); s }

class C(n: Int) extends Counted("c", n) with Sorted[Int] with AnonSorted[Int]
object O extends Counted(tag("o"), 5, 10)
class B extends Bare(3)

@main def run(): Unit =
  val c = C(3)
  println(c.bump() + c.bump())
  println(c.label + " " + c.doubled + " " + c.count)
  println(c.sort(List(3, 1, 2)).toString + " " + c.sort2(List(2, 1)))
  println(O.bump())
  val a = new Counted("anon", 1) {}
  println(a.bump() + " " + B().b)
