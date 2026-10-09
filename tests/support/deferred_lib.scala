package dfl

import scala.compiletime.deferred

// Deferred and old-style abstract givens as scalac 3.8.4 writes them, for a class of another
// build to implement (tests/classpath/jvm/jar_deferred_givens): a deferred given is a lazy given
// flagged `HASDEFAULT` without a body, an abstract method of the trait's interface, and a jar class
// that extends the trait holds scalac's implementation.
trait Ordered[A]:
  given ord: Ordering[A] = deferred
  def sorted(l: List[A]): List[A] = l.sorted

trait Named:
  given name: String = deferred
  def show: String = s"<$name>"

trait Shown:
  given shown(using n: Int): String = deferred

trait Declared:
  given decl: Int
  def twice: Int = summon[Int] * 2

class Implemented(using s: String) extends Named
