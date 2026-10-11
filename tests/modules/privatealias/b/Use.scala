package use

import privatealias.*

object Use:
  def main(args: Array[String]): Unit =
    println(Derived.make.events)
    println(Derived.size)
    println(Derived.made.n)
