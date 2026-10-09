// `v[T](args) := x`, with `v` a value whose `apply` takes type arguments and `:=` a member of
// the result, calls `:=` on `v.apply[T](args)`; an operator ending in `=` on an indexed value
// without such a member is still `update` (scalac: "autoCorrect=off", "x=on", "3").
final class Attr[-U](val name: String):
  def :=(v: U): String = s"$name=$v"
object Attr:
  inline def apply[A](inline name: String): Attr[A] = new Attr[A](name)
object Exports:
  final val VdomAttr: Attr.type = Attr
import Exports.*

@main def main(): Unit =
  println(VdomAttr[String]("autoCorrect") := "off")
  println(Exports.VdomAttr[String]("x") := "on")
  val buf = scala.collection.mutable.ArrayBuffer(1, 2)
  buf(0) += 2
  println(buf(0))
