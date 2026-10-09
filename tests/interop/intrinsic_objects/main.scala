import scala.scalajs.js.{Dynamic, JSON, Wrapper}
import _root_.scala.scalajs.js as sjs

@main def main(): Unit =
  val viaSugar = Dynamic.literal("a" -> 1, "b" -> "two")
  println(JSON.stringify(viaSugar))
  val viaApply = Dynamic.literal.apply("c" -> 3)
  println(JSON.stringify(viaApply))
  println(JSON.stringify(Dynamic.literal()))
  println(JSON.stringify(Dynamic.literal.single("k", true)))
  println(JSON.stringify(sjs.Dynamic.literal("nested" -> Dynamic.literal("x" -> 1))))
  println(Dynamic.global("Math") == _root_.js.global("Math"))
  val wrapped = Wrapper(viaSugar)
  println(wrapped("b"))
