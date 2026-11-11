package consumer

import library.values.*

object Use:
  def main(args: Array[String]): Unit =
    val step: Count = 2
    println(initial)
    println(advance(step))
    println(current)
