package rcs

import scala.language.`future`

// Under `future` a refutable pattern definition is an error unless its right side's type carries
// the exemption, which the upstream's stacked annotations have to bring across the product.
object Main {
  def main(args: Array[String]): Unit = {
    val Some(n) = Api.checked(7)
    val Some(m) = Api.marked(8)
    println(n + m)
  }
}
