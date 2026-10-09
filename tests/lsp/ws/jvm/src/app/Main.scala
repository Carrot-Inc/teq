package app

import shapes.{Circle, Square, Geometry as G}

object Main:
  def main(args: Array[String]): Unit =
    val circle = Circle(2.0)
    val shapes = List(circle, Square(3.0))
    println(G.total(shapes))
    println(circle.area)
