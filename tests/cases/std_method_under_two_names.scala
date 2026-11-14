// A method that implements both a standard library's method and a program trait's method of the same name, where
// another class joins that trait's method with an overload, answers through both types: in a class and its
// subclasses, an anonymous class, an object, an enum and a case class: the lines scalac 3.8.4 prints.
trait Own { def run(): Unit }
trait Other { def run(n: Int): Int = n }
abstract class Trigger extends Own with Other
trait Joined extends Runnable with Own
class Both extends Joined { def run(): Unit = println("class") }
class Middle extends Both
class Leaf extends Middle
object Single extends Joined { def run(): Unit = println("object") }
enum Kind extends Joined { case One; def run(): Unit = println("enum") }
case class Record(n: Int) extends Joined { def run(): Unit = println("case") }

object Main {
  def check(x: Joined): Unit = { (x: Runnable).run(); (x: Own).run() }
  def main(args: Array[String]): Unit = {
    check(new Both)
    check(new Leaf)
    check(new Joined { def run(): Unit = println("anon") })
    check(Single)
    check(Kind.One)
    check(Record(1))
  }
}
