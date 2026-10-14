package xinhu

import xinh.*
@main def run(): Unit =
  val x: API.Tag[String] = new API.Tag("ok")
  println(x.value)
