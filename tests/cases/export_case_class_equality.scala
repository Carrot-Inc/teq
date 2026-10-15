// A wildcard export of a case class's members forwards none of what a case class gets synthesized (dotty's
// `caseClassSynthesized`, `equals` among them, which a source case class declares through `scala.Equals`): the
// exporting case class keeps its own equality, as tyrian's `Location.Internal` over its `LocationDetails`.
final case class Details(path: String, port: Option[Int])
sealed trait Loc:
  def path: String
final case class Internal(details: Details) extends Loc:
  export details.*
final case class External(details: Details) extends Loc:
  export details.*
@main def run(): Unit =
  val a = Internal(Details("/p", None))
  val b = Internal(Details("/p", None))
  println(a == b)
  println(a.hashCode == b.hashCode)
  println(a.productArity)
  println(a.canEqual(b))
  println(a == External(Details("/p", None)))
  println(a.path + a.port)
