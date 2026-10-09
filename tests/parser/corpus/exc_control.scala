import scala.util.control.{ControlThrowable, NoStackTrace, NonFatal}
import scala.util.Try

final class Quiet(message: String) extends RuntimeException(message) with NoStackTrace

final class Found(val index: Int) extends ControlThrowable

def indexOf(items: Vector[String], wanted: String): Int =
  try
    for i <- items.indices do if items(i) == wanted then throw new Found(i)
    -1
  catch case f: Found => f.index

def guarded(body: => String): String =
  try body
  catch
    case NonFatal(e) => "non-fatal: " + e.getMessage
    case e: ControlThrowable => "control flow escaped"

@main def main(): Unit =
  val items = Vector("a", "b", "c")
  println(indexOf(items, "b"))
  println(indexOf(items, "z"))
  println(guarded(throw new Quiet("hush")))
  println(guarded(throw new Found(2)))
  println(guarded("plain"))
  println(Try(throw new Quiet("tried")).failed.map(_.getMessage))
  println(new Quiet("q").fillInStackTrace() eq null)
  val q = new Quiet("same")
  println(q.fillInStackTrace() eq q)
  println((NonFatal(new Quiet("x")), NonFatal(new Found(0)), NonFatal(new StackOverflowError())))
