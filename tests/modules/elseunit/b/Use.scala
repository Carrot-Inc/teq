package eub

import eua.*

def show(v: Any): String = v match
  case () => "unit"
  case other => other.toString

@main def run(): Unit =
  Steps.note("a")
  Steps.note("")
  Steps.note("b")
  println(Steps.log.reverse.mkString(","))
  println(show(Steps.pick(true)))
  println(show(Steps.pick(false)))
  println(show(Steps.last(true)))
  println(show(Steps.last(false)))
