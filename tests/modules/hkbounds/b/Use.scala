package hb

import ha.*

object Use:
  def main(args: Array[String]): Unit =
    println(lift(List(1, 2))(_ + 1))
    println(lift(Option(2))(_ * 3))
    println(largest(List(3, 1, 2)))
