// A stackable trait mixed over a class whose own method implements a standard library's method and a program
// trait's joined method runs its own body when the instance is called through the library's type, after a
// subclass's `super` call of the method: the lines scalac 3.8.4 prints.
trait Own { def run(): Unit }
trait Other { def run(n: Int): Int = n }
abstract class Trigger extends Own with Other
class Both extends Runnable with Own { def run(): Unit = println("class") }
class Sub extends Both { override def run(): Unit = { super.run(); println("sub") } }
trait Loud extends Own { abstract override def run(): Unit = { println("loud"); super.run() } }
class Stacked extends Both with Loud
object Main {
  def viaRunnable(r: Runnable): Unit = r.run()
  def main(args: Array[String]): Unit = {
    viaRunnable(() => println("lambda"))
    val s = new Sub
    s.run()
    val l = new Stacked
    viaRunnable(l)
  }
}
