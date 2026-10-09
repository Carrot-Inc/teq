// Adapted from scala3 tests/run/tryPatternMatch.scala (Apache-2.0, see tests/scala3/README.md); replaced: braces with Scala 3 syntax, `java.io.IOException` and `TimeoutException` with classes of the test, the abstract type member with a type parameter of the trait.
class IOException(message: String = null) extends Exception(message)
class TimeoutException extends Exception

object IAE:
  def unapply(e: Exception): Option[String] =
    if e.isInstanceOf[IllegalArgumentException] && e.getMessage != null then Some(e.getMessage)
    else None

object EX extends Exception:
  val msg = "a"
  class InnerException extends Exception(msg)

trait ExceptionTrait extends Exception

trait TestTrait[ExceptionType <: Exception]:
  def isExpected(e: Throwable): Boolean

  def traitTest(): Unit =
    try throw new IOException
    catch
      case e if isExpected(e) => println("success 9.2")
      case _ => println("failed 9.2")

object Test extends TestTrait[IOException]:
  def isExpected(e: Throwable): Boolean = e.isInstanceOf[IOException]

  def main(args: Array[String]): Unit =
    var a: Int = 1

    try throw new Exception("abc")
    catch
      case _: Exception => println("success 1")
      case _ => println("failed 1")

    try throw new Exception("abc")
    catch
      case e: Exception => println("success 2")
      case _ => println("failed 2")

    try throw new Exception("abc")
    catch
      case e: Exception if e.getMessage == "abc" => println("success 3")
      case _ => println("failed 3")

    try throw new Exception("abc")
    catch
      case e: Exception if e.getMessage == "" => println("failed 4")
      case _ => println("success 4")

    try throw EX
    catch
      case EX => println("success 5")
      case _ => println("failed 5")

    try throw new EX.InnerException
    catch
      case _: EX.InnerException => println("success 6")
      case _ => println("failed 6")

    try throw new NullPointerException
    catch
      case _: NullPointerException | _: IOException => println("success 7")
      case _ => println("failed 7")

    try throw new ExceptionTrait {}
    catch
      case _: ExceptionTrait => println("success 8")
      case _ => println("failed 8")

    traitTest() // test 9.2

    def testThrow(throwIt: => Unit): Unit =
      try throwIt
      catch
        case e: NullPointerException => println("NullPointerException")
        case e: IndexOutOfBoundsException => println("IndexOutOfBoundsException")
        case _: NoSuchElementException => println("NoSuchElementException")
        case _: EX.InnerException => println("InnerException")
        case IAE(msg) => println("IllegalArgumentException: " + msg)
        case _: ExceptionTrait => println("ExceptionTrait")
        case e: IOException if e.getMessage == null => println("IOException")
        case _: NullPointerException | _: IOException => println("NullPointerException | IOException")
        case EX => println("EX")
        case e: IllegalArgumentException => println("IllegalArgumentException")
        case _: ClassCastException => println("ClassCastException")

    testThrow(throw new IllegalArgumentException("abc"))
    testThrow(throw new IllegalArgumentException())
    testThrow(throw new IOException("abc"))
    testThrow(throw new NoSuchElementException())
    testThrow(throw EX)
    testThrow(throw new EX.InnerException)
    testThrow(throw new NullPointerException())
    testThrow(throw new ExceptionTrait {})
    try
      testThrow(throw new TimeoutException)
      println("TimeoutException did not escape")
    catch
      case _: TimeoutException => println("TimeoutException escaped")
