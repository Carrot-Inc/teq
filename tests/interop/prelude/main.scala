package app.widget

import app.Prelude.*

object Toggle:
  val component: Any = fc:
    val props = cls := (tw"flex ${"gap"}-2", "on" -> true, "off" -> false)
    format("%s %j", join("a", "b"), props)

@jsExport("toggle")
val toggle: Any = Toggle.component

@main def main(): Unit =
  println(js.get(Toggle.component, "displayName"))
  println(js.call(Toggle.component, "render"))
  println(app.Prelude.join("x", "y") + separator)
  val parts = List("p", "q")
  println(join(parts*))
