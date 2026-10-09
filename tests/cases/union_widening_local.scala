// A soft union widens at every inference boundary: a local val, a local var, a val inferred
// from a binder of a soft scrutinee (the binder itself keeps the union), a lambda's parameter
// taken from an inferred element type; an explicitly written union stays hard through a val,
// even beside an inferred union of the same shape.
import scala.annotation.targetName
class Box[A](val a: A)
def pick(x: Box[Int] | Box[String]): String = "union"
@targetName("pickWild")
def pick(x: Box[?]): String = "wild"

object T:
  val unused = if "a".length > 0 then new Box[Int](0) else new Box[String]("")
  val hard: Box[Int] | Box[String] = new Box[Int](1)
  val copy = hard

@main def run(): Unit =
  val x = if "ab".length > 1 then new Box[Int](1) else new Box[String]("s")
  println(pick(x))
  var y = if "ab".length > 1 then new Box[Int](2) else new Box[String]("t")
  println(pick(y))
  val bound = (if "ab".length > 1 then new Box[Int](3) else new Box[String]("u")) match
    case b => pick(b)
  println(bound)
  val rebound = (if "ab".length > 1 then new Box[Int](3) else new Box[String]("u")) match
    case b =>
      val z = b
      pick(z)
  println(rebound)
  println(List(new Box[Int](4), new Box[String]("v")).map(b => pick(b)).mkString(","))
  println(pick(T.copy))
  println(pick(T.hard))
  val same = { val k = 1; if k > 0 then new Box[Int](5) else new Box[String]("w") }
  println(pick(same))
