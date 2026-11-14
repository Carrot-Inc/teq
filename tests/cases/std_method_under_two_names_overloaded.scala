// The shapes of std_method_under_two_names where the trait that joins the standard library's method with the
// program's declares a third method of the name beside them: the lines scalac 3.8.4 prints.
trait Own { def run(): Unit }
trait Other { def run(n: Int): Int = n }
abstract class Trigger extends Own with Other
trait Joined extends Runnable with Own { def run(b: Boolean): Int = 2 }
class Both extends Joined { def run(): Unit = println("class") }
class Middle extends Both
class Leaf extends Middle
object Single extends Joined { def run(): Unit = println("object") }
enum Kind extends Joined { case One; def run(): Unit = println("enum") }
case class Record(n: Int) extends Joined { def run(): Unit = println("case") }

object Main {
  def check(x: Joined): Unit = { (x: Runnable).run(); (x: Own).run(); println(x.run(true)) }
  def main(args: Array[String]): Unit = {
    check(new Both)
    check(new Leaf)
    check(new Joined { def run(): Unit = println("anon") })
    val sam: Joined = () => println("sam")
    check(sam)
    check(Single)
    check(Kind.One)
    check(Record(1))
  }
}
