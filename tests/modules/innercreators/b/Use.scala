package ib

import ia.*

object Ob extends T:
  val base = 100
  def make(x: Int) = Box(x)

object Use:
  def main(args: Array[String]): Unit =
    val o = new O(10)
    println(o.Inner(2).get)
    locally {
      import o.*
      println(new Inner(3).get)
      println(Inner(4).get)
    }
    import o.{Inner => Renamed}
    println(Renamed(5).get)
    println(Ob.make(1).total)
    println(Ob.Box(2).total)
