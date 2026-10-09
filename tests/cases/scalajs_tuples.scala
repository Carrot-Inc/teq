//> using platform js
// `js.Tuple2` and `js.Tuple3` as Scala.js has them: a Scala tuple where one is expected through
// `fromScalaTuple2`, the other way through `toScalaTuple2`, the fields read, and the companion's
// `unapply` as a pattern.
import scala.scalajs.js

object Main:
  final case class Range(start: Int, end: Int)
  def center(c: js.Tuple2[Double, Double]): String = s"${c._1},${c._2}"
  def main(args: Array[String]): Unit =
    val lng = 1.5
    val lat = -2.25
    val jsCenter: js.Tuple2[Double, Double] = (lng, lat)
    println(center(jsCenter))
    println(center((3.0, 4.5)))
    val indices: js.Array[js.Tuple2[Int, Int]] = js.Array(js.Tuple2(0, 2), js.Tuple2(5, 7))
    println(indices.toList.map { case js.Tuple2(s, e) => Range(s, e) })
    val back: (Double, Double) = jsCenter
    println(back)
    val t3: js.Tuple3[Int, String, Boolean] = (1, "a", true)
    t3 match
      case js.Tuple3(a, b, c) => println(s"$a $b $c")
    println(t3._3)
    println(js.Array.isArray(jsCenter))
