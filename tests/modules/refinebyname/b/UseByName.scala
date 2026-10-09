package rbnb

import rbna.ByName

object UseByName:
  val original: AnyRef { def run(x: => Int): Int } = new AnyRef { def run(x: => Int): Int = x + x }
  val returned: AnyRef { def run(x: => Int): Int } = ByName.keep(original)
  def main(args: Array[String]): Unit = println(if returned.eq(original) then "byname" else "other")
