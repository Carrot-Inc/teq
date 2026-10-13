package gdoa

import scala.compiletime.deferred

// A class implementing a deferred given beside a method of its name with other parameters, read
// over the products: the implementation is the class's member in its pickle, the method its
// alternative.
trait T:
  given x: Int = deferred
class C(using i: Int) extends T:
  def x(s: String): String = s + i
