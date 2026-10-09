// jars: scala-library zio-json magnolia zio
//> using dep dev.zio::zio-json:0.9.2
// zio-json's codecs for an Either, a Set, a Map, a Vector, a BigDecimal, an Option[Long] and a
// tuple, which build through the collections' newBuilder, encoded, decoded and compared.
import zio.json.*
case class T(e: Either[Int, String], s: Set[Int], m: Map[String, Int], v: Vector[Int], big: BigDecimal, o: Option[Long], t: (Int, String)) derives JsonCodec
object Main:
  def main(args: Array[String]): Unit =
    val t = T(Left(1), Set(2), Map("k" -> 1), Vector(1, 2), BigDecimal("1.5"), Some(3L), (1, "a"))
    val j = t.toJson
    println(j)
    println(j.fromJson[T] == Right(t))
    println("""{"e":{"Right":"x"},"s":[],"m":{},"v":[],"big":2,"o":null,"t":[2,"b"]}""".fromJson[T])
