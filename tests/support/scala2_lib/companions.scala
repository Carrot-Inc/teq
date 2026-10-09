package companions

// A Scala 2 trait whose companion object the source declares first, so the pickle holds the module class `T` before
// the trait `T` (tests/classpath/jvm/scala2_companion_trait_storage): the class mixing the trait in finds the trait's
// private lazy val on the trait's symbol, not the module class's.
object T {
  def made: String = "companion"
}

trait T {
  private lazy val n: Int = { println("T lazy init"); 7 }
  def read: Int = n
}
