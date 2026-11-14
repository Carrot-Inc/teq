// A method that implements a std trait's method and a program trait's declaration of that name
// answers under both of their output names. A method of another name, which an annotation gives
// one of those output names, holds no name the bridge would stand under: the call through the
// program's trait reaches the implementation, and so does the call through the std's (scalac
// prints `tick`, `tick`).
import scala.annotation.targetName

trait Step { def run(): Unit }
trait Counted { def run(n: Int): Int = n }
abstract class Trigger extends Step with Counted
trait Timed extends Runnable with Step

class Clock extends Timed {
  def run(): Unit = println("tick")
  def label(s: String): Unit = ()
  @targetName("run$0") def label(x: Int): Unit = println("label")
}

@main def main(): Unit =
  val s: Step = new Clock
  s.run()
  val r: Runnable = new Clock
  r.run()
