package app

import a.*
import b.*

@main def run(): Unit =
  println(describe(Color.Red))
  println(describe(Color.Green))
  println(Widget("w").show)
  println(Mode.values.toList)
  println(Color.valueOf("Red").ordinal)
  println(aTop)
  println(bTop)
  val named: List[Named] = List(Color.Red, Mode.Fast, Pair(Color.Green, Mode.Slow))
  named.foreach(n => println(n.greet))
  println(Pair(Color.Red, Mode.Fast) == Pair(Color.Red, Mode.Fast))
