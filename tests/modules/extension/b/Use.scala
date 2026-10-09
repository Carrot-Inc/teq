package eb

import ea.Syntax.*
import ea.half

object Use:
  def main(args: Array[String]): Unit =
    println("hi".shout)
    println("ab".repeat(2))
    println(List(1, 2, 3).second)
    println(1 +: "x")
    println(3.0.half)
