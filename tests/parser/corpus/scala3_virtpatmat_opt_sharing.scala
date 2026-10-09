// Adapted from scala3 tests/run/virtpatmat_opt_sharing.scala (Apache-2.0, see tests/scala3/README.md); replaced: exhaustivity warning silenced with @unchecked, extends App.
/*
 * filter: It would fail on the following input
 */
object Test {
  def main(args: Array[String]): Unit = ()
  virtMatch()
  def virtMatch() = {
    (List(1, 3, 4, 7): @unchecked) match {
      case 1 :: 3 :: 4 :: 5 :: x => println("nope")
      case 1 :: 3 :: 4 :: 6 :: x => println("nope")
      case 1 :: 3 :: 4 :: 7 :: x => println(1)
    }
  }
}
