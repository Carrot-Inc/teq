// Adapted from scala3 tests/run/virtpatmat_alts.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App; `: @unchecked` silences the exhaustivity warnings scalac gives as well.
/*
 * filter: It would fail on the following input
 */
object Test {
  def main(args: Array[String]): Unit = ()
  ((true, true): @unchecked) match {
    case (true, true) | (false, false) => 1
  }

  (List(5): @unchecked) match {
    case 1 :: Nil | 2 :: Nil  => println("FAILED")
    case (x@(4 | 5 | 6)) :: Nil => println("OK "+ x)
    case 7 :: Nil  => println("FAILED")
    case Nil  => println("FAILED")
  }
}
