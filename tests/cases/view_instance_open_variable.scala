// A given conversion's view is the instance applied to the receiver, of that application's type
// with its variables (`Implicits.tryConversion`): the instance's own `B` of `conv[B]:
// Conversion[Raw, Box[B]]`, held nonvariantly, stays open for the member selected on the view
// ("no interpolation for nonvariant"), which fixes it: `put("three")` makes it a `String`, an
// expected `Box[Int]` an `Int`, a member that names it nowhere leaves it to the interpolation.
package viewinstanceopenvariable

import scala.language.implicitConversions

final case class Box[B](value: Option[B]):
  def size: Int = value.size
  def self: Box[B] = this
  def put(b: B): Box[B] = Box(Some(b))
final case class Raw(n: Int)
given conv[B]: Conversion[Raw, Box[B]] = x => Box(None)

object Main:
  def main(args: Array[String]): Unit =
    println(Raw(1).size)
    val s = Raw(2).self
    println(s)
    val p = Raw(3).put("three")
    println(p.value.map(_.length))
    val q: Box[Int] = Raw(4).self
    println(q.put(4))
