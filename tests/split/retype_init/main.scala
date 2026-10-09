package retypeinit

import retypeinit.lib.*

@main def run(): Unit =
  println("start")
  println(twice({ println("argument"); 2 }))
  println(List(1, 2).map(twice))
