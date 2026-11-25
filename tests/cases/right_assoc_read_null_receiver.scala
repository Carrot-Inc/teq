// An infix right-associative member on a null receiver whose left operand throws: the left operand is bound and
// runs first, so its exception is the one seen.
class R:
  def +:(l: Int): Int = { println("body"); l }
def boom(): Int = { println("left"); throw new IllegalArgumentException("left") }
@main def run(): Unit =
  val r: R = null
  try println(boom() +: r)
  catch case e: Throwable => println(e.getClass.getSimpleName)
