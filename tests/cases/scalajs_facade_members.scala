//> using platform js
//> using jsModuleKind es
// Scala.js's facade members a program reaches (Airstream's): `js.|` imported as Scala 3's union, `js.Math`'s `Int`
// overloads of `max` and `min`, `JSConverters`' `toJSPromise` of a future (its failure's value unwrapped) and
// `js.dynamicImport`, whose body runs in a later task, after the promises settled meanwhile.
import scala.scalajs.js
import scala.scalajs.js.|
import scala.scalajs.js.JSConverters.*
import scala.concurrent.{Future, Promise}
import scala.concurrent.ExecutionContext.Implicits.global

@main def main(): Unit =
  val n: Int = js.Math.max(3, 7)
  val d: Double = js.Math.max(1.5, 0.5)
  val u: Int | String = "five"
  println(s"$n $d $u ${js.Math.min(4, 2, 9)}")
  val p = Promise[Int]()
  p.future.toJSPromise.`then`[Unit]((v: Int) => println(s"resolved $v"))
  Future.failed[Int](new IllegalStateException("no")).toJSPromise.`then`[Unit]((v: Int) => println(v), (e: Any) => println(s"rejected ${e.asInstanceOf[Throwable].getMessage}"))
  js.dynamicImport { println("imported"); 42 }.`then`[Unit]((v: Int) => println(s"import gave $v"))
  println("sync end")
  p.success(1)
