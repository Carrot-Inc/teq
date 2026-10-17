package rcr

import scala.language.`future`

// Under `future` a refutable pattern definition is an error unless its right side's type carries
// the exemption, which the upstream's result has to bring across the product.
object Main {
  def main(args: Array[String]): Unit = {
    val Some(n) = Api.checked(7)
    println(n)
  }
}
