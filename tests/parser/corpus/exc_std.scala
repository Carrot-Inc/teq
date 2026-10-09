import scala.util.control.NonFatal
import scala.util.Try

def attempt(label: String)(body: => Any): Unit =
  val outcome =
    try "value " + body.toString
    catch
      case e: ArithmeticException => "arithmetic: " + e.getMessage
      case e: NumberFormatException => "number: " + e.getMessage
      case e: NoSuchElementException => "missing: " + e.getMessage
      case e: IndexOutOfBoundsException => "index: " + e.getMessage
      case e: UnsupportedOperationException => "unsupported: " + e.getMessage
      case e: IllegalArgumentException => "argument: " + e.getMessage
      case e: MatchError => "match: " + e.getMessage
      case e: NotImplementedError => "todo: " + e.getMessage
      case e: AssertionError => "assertion: " + e.getMessage
      case e: RuntimeException => "runtime: " + e.getMessage
  println(label + " -> " + outcome)

def classify(t: Throwable): String = t match
  case _: NumberFormatException => "NumberFormatException"
  case _: IllegalArgumentException => "IllegalArgumentException"
  case _: RuntimeException => "RuntimeException"
  case _: Exception => "Exception"
  case _: Error => "Error"
  case _ => "Throwable"

def fatal(t: Throwable): String = t match
  case NonFatal(e) => "non-fatal " + classify(e)
  case e => "fatal " + classify(e)

@main def main(): Unit =
  val zero = 0
  val empty = List.empty[Int]
  val none: Option[Int] = None
  attempt("division")(10 / zero)
  attempt("long division")(10L / zero.toLong)
  attempt("remainder")(10 % zero)
  attempt("toInt")("12x".toInt)
  attempt("toLong")("".toLong)
  attempt("toDouble")("abc".toDouble)
  attempt("Option.get")(none.get)
  attempt("head")(empty.head)
  attempt("tail")(empty.tail)
  attempt("last")(empty.last)
  attempt("apply")(List(1, 2)(5))
  attempt("vector apply")(Vector(1)(-1))
  attempt("map apply")(Map("a" -> 1)("b"))
  attempt("reduce")(empty.reduce(_ + _))
  attempt("max")(empty.max)
  attempt("iterator")(empty.iterator.next())
  attempt("sys.error")(sys.error("boom"))
  attempt("require")(require(zero > 0, "zero is not positive"))
  attempt("require plain")(require(zero > 0))
  attempt("assert")(assert(zero == 1, "zero is not one"))
  attempt("???")(???)
  attempt("match")((zero: Any) match { case s: String => s })
  attempt("charAt")("abc".charAt(9))
  attempt("fromOrdinal")(Color.fromOrdinal(7))
  attempt("valueOf")(Color.valueOf("Pink"))
  attempt("fine")(zero + 1)
  println(classify(new NumberFormatException("n")))
  println(classify(new IllegalStateException("s")))
  println(classify(new Exception("e")))
  println(classify(new AssertionError("a")))
  println(classify(new Throwable("t")))
  println(fatal(new RuntimeException("r")))
  println(fatal(new StackOverflowError()))
  println(fatal(new InterruptedException()))
  println(fatal(new OutOfMemoryError()))
  println(Try(10 / zero).failed.get.getMessage)
  println(Try("x".toInt).isFailure)
  println(Try(none.get).failed.map(_.getMessage))
  println(Try(1).filter(_ > 5).failed.get.getMessage)
  println(Try(1).failed.failed.get.getMessage)
  println(new MatchError(zero).getMessage)
  println(new NoSuchElementException("gone").toString)
  println(new ArithmeticException("/ by zero").toString)
  println(new NotImplementedError().getMessage)
  println(new AssertionError("failed", new RuntimeException("cause")).getCause.getMessage)

enum Color:
  case Red, Green
