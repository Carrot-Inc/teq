package rca

import scala.language.`future`

// Under `future` a refutable pattern definition is an error: the component is checked.
object Main {
  def main(args: Array[String]): Unit = {
    val E(Some(n)) = 7
    println(n)
  }
}
