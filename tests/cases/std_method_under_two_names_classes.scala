// A class's method that implements both a standard library's method and a program trait's method of the same name,
// which another class joins with an overload: declared by the class beside an overload of its own, inherited from a
// trait, and as a function's `apply`: each answers through both types, as under scalac 3.8.4.
trait Own { def run(): Unit }
trait Other { def run(n: Int): Int = n }
abstract class Trigger extends Own with Other

class Direct extends Runnable with Own {
  def run(): Unit = println("direct")
  def run(b: Boolean): Int = 3
}

trait Ready extends Runnable { def run(): Unit = println("inherited") }
class FromTrait extends Ready with Own

trait Step { def apply(n: Int): Int }
trait Named { def apply(s: String): Int = 9 }
abstract class StepTrigger extends Step with Named
class Inc extends (Int => Int) with Step { def apply(n: Int): Int = n + 1 }

object Main {
  def main(args: Array[String]): Unit = {
    val d = new Direct
    (d: Runnable).run()
    (d: Own).run()
    println(d.run(true))
    val t = new FromTrait
    (t: Runnable).run()
    (t: Own).run()
    val inc = new Inc
    println((inc: Int => Int)(2))
    println((inc: Step).apply(3))
  }
}
