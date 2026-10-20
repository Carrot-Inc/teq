// `data.copyurl := v`: scalatags' `object data extends DataAttribute(...)`, a Dynamic,
// declared in a trait and selected through an object that extends it.
import scala.language.dynamics

class DataAttribute(sections: List[String]) extends scala.Dynamic:
  final def selectDynamic(s: String) = new DataAttribute(s :: sections)
  final def :=(v: String): String = sections.reverse.mkString("-") + "=" + v

trait GlobalAttrs:
  object data extends DataAttribute(List("data"))

object st extends GlobalAttrs

@main def run() =
  println(st.data.copyurl := "y")
