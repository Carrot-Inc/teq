package alb

import ala.*

def keep[U](u: U): U = u

@main def run(): Unit =
  val m: Defs.Memo[Int] = Defs.memo[Int]("m")
  println(keep(m).name)
