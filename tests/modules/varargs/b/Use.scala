package vb

import va.*

object Use:
  def main(args: Array[String]): Unit =
    println(Sum.all(1, 2, 3))
    println(Sum.all(List(4, 5)*))
    println(Sum.labelled("l:", "a", "b"))
    println(new Bag("x", "y").items.size)
