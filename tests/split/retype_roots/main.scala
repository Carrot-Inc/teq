package roots

import roots.data.Pair
import roots.other.Other
import roots.use.Use

object Main:
  def main(args: Array[String]): Unit =
    println(Use.label + " " + Use.first(Pair(1, 2)) + " " + Use.named(Other()) + " " + Other().label)
