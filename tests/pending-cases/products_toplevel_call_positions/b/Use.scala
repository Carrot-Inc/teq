// A downstream of a/ as tests/modules builds it. b/ calls
// a/'s top-level `valueOf` (`valueOf(Keys.Name)`). scalac
// 3.8.4 prints the .expected. b/'s pickle is to be the same
// in teq's whole build and in its build over a/'s products,
// the positions of the call's function and of its qualifier
// too (tests/modules' classes, subset, check-lean-whole).
package zdb

import zda.*

object Use:
  def main(args: Array[String]): Unit =
    val n: String = valueOf(Keys.Name)
    val c: Int = valueOf(Keys.Count)
    println(n)
    println(c)
