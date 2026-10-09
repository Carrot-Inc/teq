// Adapted from scala3 tests/run/finally.scala (Apache-2.0, see tests/scala3/README.md); replaced: Scala 2 syntax with Scala 3 syntax, `App` with a main method; the cases with `return` are left out.
object Test:
  // test that finally is not covered by any exception handlers.
  def throwCatchFinally: Unit =
    try bar
    catch case e: Throwable => println(e)

  // test that finally is not covered by any exception handlers.
  def bar: Unit =
    try println("hi")
    catch case e: Throwable => println("SHOULD NOT GET HERE")
    finally
      println("In Finally")
      throw new RuntimeException("ouch")

  // throw in catch (finally is executed, exception propagated)
  def throwCatch: Unit =
    try throw new Exception
    catch
      case e: Throwable =>
        println(e)
        throw e
    finally println("in finally")

  // nested finally blocks with return values
  def nestedFinalies: Int =
    try
      try
        1
      finally
        println("in finally 1")
    finally
      println("in finally 2")

  def nestedFinallyBlocks: Int =
    var i = 0
    try
      try
        println("in body")
      finally
        println("in finally 1")
        i += 1
    finally
      println("in finally 2")
      i += 1
    i

  def throwInBody: Unit =
    try
      throw new Exception("in body")
    finally println("in finally")

  def throwInFinally(): Unit =
    try
      println("in body")
    finally throw new Exception("in finally")

  def valueOfTry: Int =
    try 5
    finally println("finally with value")

  def test(name: String)(body: => Unit): Unit =
    println("----------- " + name)
    try body
    catch case e: Throwable => println("CAUGHT: " + e)
    println()

  def main(args: Array[String]): Unit =
    test("throw-catch-finally")(throwCatchFinally)
    test("throw-catch")(throwCatch)
    test("nested-finalies")(println(nestedFinalies))
    test("nested-finally-blocks")(println(nestedFinallyBlocks))
    test("throw-in-body")(throwInBody)
    test("throw-in-finally")(throwInFinally())
    test("value-of-try")(println(valueOfTry))
