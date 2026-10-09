//> using platform js
// `js.Promise`'s executor takes a resolve that accepts a value or a thenable of it, and functions
// whose results are ignored, as Scala.js declares it.
import scala.scalajs.js

@main def run(): Unit =
  val inner = js.Promise.resolve[Int](20)
  val p = new js.Promise[Int]((resolve: js.Function1[Int | js.Thenable[Int], ?], reject: js.Function1[Any, ?]) => resolve(inner))
  p.`then`[Unit](n => println(n + 1))
  val q = new js.Promise[String]((resolve, _) => resolve("done"))
  q.`then`[Unit](s => println(s))
