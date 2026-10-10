//> using platform js
//> using jsVersion 1.22.0

import scala.scalajs.js

// A cast to a JavaScript type is never tested, as under Scala.js: a Scala function cast to a JS
// function type of another arity, and a JS value cast to js.Dynamic, are the values themselves.
object Main:
  def main(args: Array[String]): Unit =
    val mod: Int => Int = _ + 1
    val jsFn1 = mod: js.Function1[Int, Int]
    val jsFn2 = jsFn1.asInstanceOf[js.Function2[Int, String, Int]]
    println(jsFn2(41, "ignored"))
    val raw: js.Any = js.Dynamic.literal(a = 1)
    val d = raw.asInstanceOf[js.Dynamic]
    println(d.a)
    val o = raw.asInstanceOf[js.Object]
    println(js.Object.keys(o).mkString(","))
    val anyFn: Any = mod
    val f2 = anyFn.asInstanceOf[js.Function2[Int, String, Int]]
    println(f2 != null)
    val f0: Any = () => 1
    try { f0.asInstanceOf[Int => Int]; println("arity passed") }
    catch case _: ClassCastException => println("arity CCE")
    println(("s": Any).isInstanceOf[Int => Int])
