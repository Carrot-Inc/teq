package app

import fac.Facade.*

object Main:
  def main(args: Array[String]): Unit =
    println(greet("x"))
    println(twice(21))
    println(own)
