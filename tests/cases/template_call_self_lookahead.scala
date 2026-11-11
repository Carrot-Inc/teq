object Invoke:
  def apply(f: () => Unit): Unit = f()
class BracedCall:
  Invoke { () =>
    println("braced")
  }
object AcceptInt:
  def apply(x: Int): Unit = println(x)
class ColonCall:
  AcceptInt:
    22
@main def templateCalls(): Unit =
  new BracedCall
  new ColonCall
