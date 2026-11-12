// Local lazy vals in a `finally`, a `catch` case and a `try`'s body: the backend writes a finalizer twice, on the
// normal exit and in the catch-all handler (dotty's `emitFinalizer`), and each copy makes its own holder as dotty's
// `transformStats` puts it at the head of the copied statements; a `return` through the finalizer, a lambda and a
// local def reading its lazy val, and a finalizer inside a finalizer.
object Main:
  def f(fail: Boolean): Unit =
    try
      if fail then throw new Exception("boom")
      println("body")
    finally
      lazy val x = { println("init"); 42 }
      println(x)

  def g(fail: Boolean): Int =
    try
      lazy val a = { println("a init"); 1 }
      if fail then throw new IllegalStateException("g")
      a + a
    catch
      case e: IllegalStateException =>
        lazy val b = { println("b init " + e.getMessage); 2 }
        b + b
    finally
      lazy val c = { println("c init"); 3 }
      println("finally " + (c + c))

  def h(n: Int): Int =
    try
      if n > 0 then return n
      -1
    finally
      lazy val d = { println("d init"); n * 10 }
      val read = () => d
      def twice(): Int = d + d
      println("h " + read() + " " + twice())

  def k(): Unit =
    try
      try println("inner body")
      finally
        lazy val e = { println("e init"); 5 }
        println(e)
    finally
      lazy val f = { println("f init"); 6 }
      println(f + f)

  def main(args: Array[String]): Unit =
    f(false)
    try f(true) catch case _: Exception => println("caught")
    println(g(false))
    println(g(true))
    println(h(3))
    println(h(0))
    k()
    try
      try throw new RuntimeException("outer")
      finally
        lazy val z = { println("z init"); 7 }
        println(z)
    catch case e: RuntimeException => println("caught " + e.getMessage)
