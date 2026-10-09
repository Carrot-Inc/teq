//> using platform js
// `js.ThisFunctionN` is a JavaScript `function` that passes its `this` as the first parameter of
// the Scala function it is made from; applied, it is called with the first argument as `this`.
import scala.scalajs.js

@main def run(): Unit =
  val named: js.ThisFunction1[js.Dynamic, String, String] = (self: js.Dynamic, greeting: String) => s"$greeting, ${self.name}"
  val obj = js.Dynamic.literal(name = "Ada", greet = named)
  println(obj.greet("Hello"))
  println(named(js.Dynamic.literal(name = "Bob"), "Hi"))
  val bare: js.ThisFunction0[js.Dynamic, Int] = js.ThisFunction.fromFunction1((self: js.Dynamic) => self.n.asInstanceOf[Int] * 2)
  println(js.Dynamic.literal(n = 21, twice = bare).twice())
  val back: js.Dynamic => Int = bare
  println(back(js.Dynamic.literal(n = 5)))
