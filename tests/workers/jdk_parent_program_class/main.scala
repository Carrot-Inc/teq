// jars: scala-library jdkparent-lib
// A class of the program below a library class whose parent is a JDK class the std does not
// declare, an extension call on the library class (a member lookup that finds nothing first), and
// a result type inferred as the least upper bound of the library class and `Point2D`.
class Mine(a: Int, b: Int) extends jdkparent.Spot(a, b)
extension (p: jdkparent.Spot) def spotX: Int = p.x
object U:
  def miss(p: jdkparent.Spot): Int = p.spotX
  def pick(b: Boolean, p: jdkparent.Spot, q: java.awt.geom.Point2D) = if b then p else q
@main def run(): Unit =
  val q = new jdkparent.Spot(1, 2)
  val r: java.awt.geom.Point2D = new java.awt.Point(5, 6)
  println(List(U.miss(q), U.pick(true, q, r).getClass.getName, U.pick(false, q, r).getClass.getName).mkString(" "))
