class Test(using t: Int)
object Test:
  given Int = 3
  def apply() = new Test(???)
trait TC[A]
def summonTC[A](a: A)(using TC[A]) = ()
given TC[Int] with {}
@main def run(): Unit =
  val maybeInt: Int = 1
  summonTC(maybeInt)
  println(1)
