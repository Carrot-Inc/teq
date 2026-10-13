package ginb

import gina.O.given

@main def run(): Unit =
  println(summon[Int])
  import gina.E.given
  println(summon[Long])
  println(summon[Long])
  println(gina.E.count)
