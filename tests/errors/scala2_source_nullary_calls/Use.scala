// A method with an empty parameter list called without it: scalac 3.8.4 warns where the method's class is of a
// source under `import scala.language.`2.13`` (its own or inherited by a class of this source), and refuses the
// call on a class of a source of this version in the same build; the 2.13 class's alphanumeric method is an
// operator without a warning.
// expect: Use.scala:17:18: warning: method next must be called with () argument
// expect: Use.scala:18:27: warning: method label must be called with () argument
// expect: Use.scala:20:22: error: method next in class Tally must be called with () argument
// absent: Use.scala:17:18: error
// absent: Use.scala:18:27: error
// absent: Use.scala:19:
import oldsrc.*
import newsrc.*

class Derived extends Base

object Use {
  def old: Int = new Counter().next
  def inherited: String = new Derived().label
  def infix: Int = (new Meter(1) plus new Meter(2)).n
  def current: Int = new Tally().next
}
