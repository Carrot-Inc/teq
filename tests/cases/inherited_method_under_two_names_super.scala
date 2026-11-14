// A class that inherits, from a trait below a standard library's type, its implementation of a program trait's
// method that another class joins with an overload answers through the program trait after a stackable trait's
// `super` call of the name elsewhere: the lines scalac 3.8.4 prints.
trait Own { def run(): Unit }
trait Other { def run(n: Int): Int = n }
abstract class Trigger extends Own with Other
trait Ready extends Runnable { def run(): Unit = println("ready") }
class FromTrait extends Ready with Own
class Plain extends Own { def run(): Unit = println("plain") }
trait Loud extends Own { abstract override def run(): Unit = { println("loud"); super.run() } }
class Stacked extends Plain with Loud
object Main {
  def viaOwn(o: Own): Unit = o.run()
  def main(args: Array[String]): Unit = {
    viaOwn(new Plain)
    val l = new Stacked
    l.run()
    viaOwn(new FromTrait)
  }
}
