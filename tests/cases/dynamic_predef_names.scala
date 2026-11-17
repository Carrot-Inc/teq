// `ensuring`, `formatted` and `→`, which `scala.Predef` gives every value, come before the dynamic rewrite, as a
// conversion does: on a value whose class derives from `scala.Dynamic` they are Predef's, not dynamic calls.
import scala.language.dynamics

class Plain(val tag: String) extends Dynamic:
  def selectDynamic(name: String): String = s"$tag.select($name)"
  def applyDynamic(name: String)(args: Any*): String = s"$tag.apply($name)(${args.mkString(",")})"
  override def toString: String = tag

@main def run(): Unit =
  val p = new Plain("p")
  println(p.ensuring(true))
  println(p.ensuring(_.tag == "p"))
  println(p.formatted("[%s]"))
  println(p → 1)
  println(p.other(1))
