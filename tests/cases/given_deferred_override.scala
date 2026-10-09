// An explicit implementation of a deferred given needs `override`, a parameterized one needs one,
// which the search never makes ("since that given is parameterized"), and an implementation
// in a trait further down serves every class below it.
import scala.compiletime.deferred

trait T:
  given x: Int = deferred
  given list[A]: List[A] = deferred
  def both = s"$x ${list[String]}"

class C extends T:
  override given x: Int = 9
  override given list[A]: List[A] = Nil

trait U extends T:
  override given x: Int = 10
object Ints:
  given Int = 1
  class D extends U:
    override given list[A]: List[A] = List.empty[A]

@main def run(): Unit =
  println(C().both)
  println((new Ints.D).both)
  println(C().list[Int].isEmpty)
