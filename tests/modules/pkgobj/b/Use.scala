package pb

import pa.{*, given}
import pa.nested.*

object Use:
  def main(args: Array[String]): Unit =
    val n: Name = "you"
    println(greet(n) + answer)
    val i: Id = helper(2)
    println(i)
    println(summon[Name])
