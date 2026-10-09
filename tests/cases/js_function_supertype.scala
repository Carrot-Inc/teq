//> using platform js
// `js.Function` is the type of JavaScript functions; a Scala function is one on JavaScript, so it
// meets a `js.Function` bound and is a `js.Object`.
import scala.scalajs.js

def keep[F <: js.Function](f: F): F = f

@main def run(): Unit =
  val g: js.Function1[Int, Int] = x => x + 1
  println(keep(g)(2))
  val o: js.Object = g
  println(js.typeOf(o))
  println(g.asInstanceOf[js.Function].length)
