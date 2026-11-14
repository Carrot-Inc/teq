// A class whose own method implements a standard library's method and a program trait's method that another class
// joins with an overload answers through the trait after a subclass's and a stackable trait's `super` call of it:
// the lines scalac 3.8.4 prints.
trait Own { def run(): Unit }
trait Other { def run(n: Int): Int = n }
abstract class Trigger extends Own with Other
class Both extends Runnable with Own { def run(): Unit = println("class") }
class Sub extends Both { override def run(): Unit = { super.run(); println("sub") } }
trait Loud extends Own { abstract override def run(): Unit = { println("loud"); super.run() } }
class Stacked extends Both with Loud
object Main {
  def viaOwn(o: Own): Unit = o.run()
  def main(args: Array[String]): Unit = {
    viaOwn(new Sub)
    val l = new Stacked
    l.run()
    viaOwn(new Both)
  }
}
