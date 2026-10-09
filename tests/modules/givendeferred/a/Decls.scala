package gda

import scala.compiletime.deferred

// Deferred givens across module boundaries: each declaration is pickled as scalac's (a lazy given
// flagged `HASDEFAULT`, without a body, a def where it has parameters) and its trait's interface
// declares an abstract method, so that a class of a downstream module implements it by its own
// search, scalac's or teq's, and one that inherits an implementation keeps it.
trait Ordered[A]:
  given ord: Ordering[A] = deferred
  def sorted(l: List[A]): List[A] = l.sorted

trait Named:
  given name: String = deferred
  def show: String = s"<$name>"

trait Shown:
  given shown(using n: Int): String = deferred

object Counter:
  var made: Int = 0

trait Boxed[A]:
  given value: A = deferred
  def get: A = value

// Reachable in its package alone: the implementation keeps the qualifier in its pickle.
trait Qualified:
  protected[gda] given q: Int = deferred
