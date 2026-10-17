package rci

import scala.language.`future`

// Under `future` a refutable pattern definition is an error unless its right side's type carries
// `RuntimeCheckedAnnot` (`Checking.checkIrrefutable`): the upstream wrapper's expansion keeps it,
// whichever compiler wrote the upstream's products.
object Main {
  def main(args: Array[String]): Unit = {
    val Some(n) = Checked.checked(Option(7))
    println(n)
  }
}
