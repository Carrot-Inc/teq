// JSCodeGen#genClosure boxes a closure's Unit result (JSCodeGen.scala:3700, makePrimitiveBox 3955–3956): the body run, `undefined`
// returned, for a js.FunctionN and a Scala function alike. The expectation is scalac 3.8.4's under Scala.js 1.22.0
// (`scala-cli run --js --js-version 1.22.0 --scala 3.8.4`), not written by hand.
import scala.scalajs.js
import scala.scalajs.js.annotation.*

object G:
  @js.native @JSGlobal("parseInt") def p(s: String): Unit = js.native

@main def run(): Unit =
  val f: js.Function0[Unit] = () => G.p("7")
  println(js.typeOf(f.asInstanceOf[js.Function0[Any]]()))
  val g: () => Unit = () => G.p("7")
  println(js.typeOf(g.asInstanceOf[() => Any]()))
  val h: () => Unit = () => println("x")
  h()
  val k: Int => Unit = x => println(x)
  List(1, 2).foreach(k)
