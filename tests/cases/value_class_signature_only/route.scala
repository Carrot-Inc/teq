// A value class of another file that this one names in signatures alone: results, parameters,
// a field, a list's element, a value given where `Any` is wanted. Under scala-library's erasure
// on the JVM each of them is the underlying `double`, and the box is made and opened where a
// reference is wanted; the lean std keeps the class in them.
import units.Meters

object Route:
  def leg: Meters = Meters.of(2.5)
  def pass(m: Meters): Meters = m
  var last: Meters = leg
  def boxed: Any = pass(leg)
  def all: List[Meters] = List(leg, pass(Meters.of(4.25)))
  def first: Meters = all.head

@main def run(): Unit =
  println(Route.boxed)
  println(Route.all)
  Route.last = Route.first
  val kept = Route.pass(Route.last)
  println(List(kept, Route.leg).size)
  println(Route.boxed.getClass.getName)
