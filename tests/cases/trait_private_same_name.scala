// A private member is called as itself (Scala.js's `JSCodeGen.genApplyMethodMaybeStatically`, a
// private namespace per class): a trait's private next to a private of the same name in the trait
// extending it, or in a sibling trait the class mixes in, is the one its own trait's methods call.
trait S[U]:
  def enableS: U = enableFlag[Int]("s")
  private def enableFlag[X](tag: String): U = { println("S." + tag); this.asInstanceOf[U] }
trait T[U]:
  def enableT: U = enableFlag[Int]("t")
  private def enableFlag[X](tag: String): U = { println("T." + tag); this.asInstanceOf[U] }
trait F[U] extends S[U] with T[U]:
  def enableF: U = enableFlag[Int]("f")
  private def enableFlag[X](tag: String): U = { println("F." + tag); this.asInstanceOf[U] }
final class C extends F[C]
@main def run(): Unit =
  val c = C()
  c.enableS.enableT.enableF
  println("ok")
