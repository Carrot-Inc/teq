package app

import q.Names.*

object A:
  val y = "A.y"
object B:
  val y = "B.y"

object Main:
  def run(): Unit =
    println(p.Show.foo)
    println(p.Show.bar)
    println(x)
    def inner =
      import B.*
      y
    println(inner)
    def named =
      import A.*
      import B.y
      y
    println(named)
    a.b.c.Chain.run()
