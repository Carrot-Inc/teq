package dpb

import dpa.*

object Use:
  def main(args: Array[String]): Unit =
    val n: String = valueOf(Keys.Name)
    val c: Int = valueOf(Keys.Count)
    println(n + c)
