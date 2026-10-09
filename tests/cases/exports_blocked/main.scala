package app

import util.Prelude.*

@main def run(): Unit =
  println(own(1))
  println(helper(5))
  println(util.DeskAppUtils.helper(6))
