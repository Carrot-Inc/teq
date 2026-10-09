//> using platform js
// JavaScript's `WeakRef` and `FinalizationRegistry` as Scala.js declares them.
import scala.scalajs.js

@main def main(): Unit =
  val target = js.Dynamic.literal(name = "kept")
  val ref = new js.WeakRef(target)
  println(ref.deref().map(_.asInstanceOf[js.Dynamic].name.asInstanceOf[String]))
  val registry = new js.FinalizationRegistry[js.Object, String, js.Object]((held: String) => println(held))
  val token = js.Dynamic.literal()
  registry.register(target, "gone", token)
  registry.register(js.Dynamic.literal(), "other")
  println(registry.unregister(token))
  println(registry.unregister(token))
