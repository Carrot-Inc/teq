// jars: scala-library zio-json magnolia zio
//> using dep dev.zio::zio-json:0.9.2
// zio-json's number codecs from the jar: doubles, floats, longs and big decimals, which write
// through `SafeNumbers` and a `java.lang.StringBuilder`, and read back; decoded values are shown
// re-encoded, since doubles print differently on JavaScript.
import zio.json.*

case class Reading(d: Double, f: Float, l: Long, big: BigDecimal, small: Double) derives JsonCodec

object Main:
  def main(args: Array[String]): Unit =
    val r = Reading(1.5, 0.25f, 1234567890123L, BigDecimal("12345.678"), 1e-7)
    val json = r.toJson
    println(json)
    println(json.fromJson[Reading].map(_.toJson))
    println(json.fromJson[Reading] == Right(r))
    println(List(0.1, -2.0, 1e21, 3.0e-5).toJson)
    println("[1.25,2]".fromJson[List[Double]].map(_.toJson))
    println("\"x\"".fromJson[Double])
