// Adapted from scala3 tests/run/virtpatmat_casting.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
object Test {
  def main(args: Array[String]): Unit = ()
  println(List(1,2,3) match {
    case Nil => List(0)
// since the :: extractor's argument must be a ::, there has to be a cast before its unapply is invoked
    case x :: y :: z :: a :: xs => xs ++ List(x)
    case x :: y :: z :: xs => xs ++ List(x)
    case _ => List(0)
  })
}
