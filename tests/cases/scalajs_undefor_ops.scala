//> using platform js
// `js.UndefOr[A]`'s operations on a union with `Unit`, which scalac finds through `js.internal.UnitOps` in
// the implicit scope of `Unit`; `js.UndefOr[Any]` stays the union `Any | Unit` for them.
import scala.scalajs.js

object Main:
  def describe(error: js.UndefOr[Any]): String = error.toOption match
    case Some(e) => s"some $e"
    case None => "none"
  val cb: (Int, js.UndefOr[Any]) => String = (n: Int, e: js.UndefOr[Any]) => s"$n ${e.toOption}"
  def main(args: Array[String]): Unit =
    println(describe(js.undefined))
    println(describe("boom"))
    println(cb(1, 2))
    println(cb(2, js.undefined))
    More.run()

object More:
  def run(): Unit =
    val y: String | Unit = "s"
    println(y.toOption)
    val z: js.UndefOr[Int] = js.undefined
    println(z.toOption)
    println(z.getOrElse(7))
    val w: js.UndefOr[Int] = 5
    println(w.map(_ + 1).toOption)
