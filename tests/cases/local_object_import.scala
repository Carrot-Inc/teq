// A local object with an abstract type member and a given of it, imported from, as izumi-reflect
// reads its pre-2.3.0 lambda parameters.
trait Show[A]:
  def show(a: A): String

object Main:
  def run(xs: List[String]): String =
    object Param:
      type Param <: String
      implicit val showParam: Show[Param] = new Show[Param]:
        def show(a: Param): String = "<" + a + ">"
    import Param.{Param, showParam}
    val ps: List[Param] = xs.map(_.asInstanceOf[Param])
    ps.map(p => summon[Show[Param]].show(p)).mkString
  def main(args: Array[String]): Unit =
    println(run(List("a", "b")))
