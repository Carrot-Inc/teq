// jars: scala-library
// A `deferred` given (SIP-64) is abstract, which a class implements with `override given`, where a concrete given is
// final and overriding one is an error (tests/errors/override_given_final): the class holds the implementing given.
import scala.compiletime.deferred

trait Sorter:
  given ord: Ordering[Int] = deferred
  def sorted(l: List[Int]): List[Int] = l.sorted

class Down extends Sorter:
  override given ord: Ordering[Int] = Ordering.Int.reverse

object Up extends Sorter:
  override given ord: Ordering[Int] = Ordering.Int

@main def run(): Unit =
  println(Down().sorted(List(2, 3, 1)))
  println(Up.sorted(List(2, 3, 1)))
