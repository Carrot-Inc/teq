package bab

import bao.*

// Named arguments written out of order are evaluated in the order written: the call binds the first written
// before the object named second is loaded, on every target, over the upstream's sources and over its products.
object Main:
  def main(args: Array[String]): Unit =
    println("named arguments out of order, an object last"); pair(b = effect("b"), a = Bare)
    println("end")
