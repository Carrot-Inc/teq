// jars: zio-json magnolia zio
// A generic case class's codec derived under a `JsonCodec` context bound: the encoder of
// `Box[String]` is the codec's, not zio-json's `iterable[A, T[X] <: Iterable[X]]`, whose bound
// a `Box` does not meet (the search once took it and encoded the box as a collection).
//> using dep dev.zio::zio-json:0.9.2
import zio.json.*

final case class Box[A](a: A, n: Int)
object Box:
  given [A: JsonCodec]: JsonCodec[Box[A]] = DeriveJsonCodec.gen
final case class Pair[K, V](k: K, v: V)
object Pair:
  given [K: JsonCodec, V: JsonCodec] => JsonCodec[Pair[K, V]] = DeriveJsonCodec.gen
final case class Wrapper(box: Box[Int], pairs: List[Pair[String, Boolean]])
object Wrapper:
  given JsonCodec[Wrapper] = DeriveJsonCodec.gen

object Main:
  def main(args: Array[String]): Unit =
    println(Box("x", 1).toJson)
    println(Pair(1, "v").toJson)
    println(Wrapper(Box(7, 2), List(Pair("a", true))).toJson)
    println("""{"a":"y","n":3}""".fromJson[Box[String]])
    println("""{"box":{"a":1,"n":2},"pairs":[{"k":"z","v":false}]}""".fromJson[Wrapper])
    println(List(Box("q", 0)).toJson)
