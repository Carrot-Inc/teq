// A downstream of a/ as tests/modules builds it. b/ calls a/'s top-level `valueOf` (`valueOf(Keys.Name)`). scalac 3.8.4
// prints the .expected, and so does each of teq's runs. b/'s pickle differs between the whole build and the build over
// a/'s products in positions alone: the whole build gives the call's function its span and its qualifier a point at the
// start, the build over the products neither span (tests/modules' classes, subset, check-lean-whole). The same on
// integration 428c920b0c; tests/modules/dependent shows it since 3a451f6219 pickles its `n + c` body, which the writer
// withheld before. Found by the workers exports helper (push 9).
package zdb

import zda.*

object Use:
  def main(args: Array[String]): Unit =
    val n: String = valueOf(Keys.Name)
    val c: Int = valueOf(Keys.Count)
    println(n)
    println(c)
