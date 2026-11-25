//> using platform jvm
// std: scala-library
// The names of the closures an op-assignment makes, printed from inside each (its method's name, the stack's own
// frame): `b(i) += 2` over a class whose `apply` and `update` take `i` by name types `b.update(i, b.apply(i) + 2)`,
// whose tree holds the argument twice, so scalac makes a closure at each place, two methods; a function literal
// passed so is lifted to a val first, one method; a lambda in a `finally`, which the back end writes on each exit,
// one method, and the op-assignment's two in a `finally`, two methods, each the one of its place in every copy.
object Main {
  def here(): String = Thread.currentThread.getStackTrace()(2).getMethodName

  class ByName {
    def apply(i: => String): Int = { println("apply " + i); 10 }
    def update(i: => String, v: Int): Unit = println("update " + i + " " + v)
  }
  class Fun {
    def apply(f: Int => String): Int = { println("apply " + f(1)); 1 }
    def update(f: Int => String, v: Int): Unit = println("update " + f(2) + " " + v)
  }

  def main(args: Array[String]): Unit = {
    val b = new ByName
    b(here()) += 2
    val fun = new Fun
    fun(x => here()) += 3
    try println("body")
    finally { val g = () => here(); println("finally " + g()) }
    try println("body") finally { b(here()) += 4 }
  }
}
