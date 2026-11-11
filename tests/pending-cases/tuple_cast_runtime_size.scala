// scalac 3.8.4 prints the .expected. A Tuple3 passes the cast to `Int *: Int *: EmptyTuple`, which scalac erases to
// Product (tupleArity is -1 for a chain ending in the EmptyTuple alias, TypeErasure.scala:82-95), and `size` is
// `runtime.Tuples.size` of the value (Tuple.scala:66-67), 3. teq's interpreter, Scala.js and JVM builds print 2, the
// static size. `._2` throws ClassCastException on every one, as scalac's does. The same on integration a097de53d9.
// On master efddb38de3 the interpreter and Scala.js print 2 as well, and the JVM build throws ClassCastException at
// the cast. Found by reviewer #15 of the workers exports helper (push 9).
def head(x: Int *: Int *: EmptyTuple): Int = x._1

@main def run(): Unit =
  try println(((2, 3, 4): Any).asInstanceOf[Int *: Int *: EmptyTuple])
  catch case _: ClassCastException => println("CCE1")
  try
    val t = ((2, 3, 4): Any).asInstanceOf[Int *: Int *: EmptyTuple]
    println(t.size)
  catch case _: ClassCastException => println("CCE2")
  try println(((2, 3, 4): Any).asInstanceOf[Int *: Int *: EmptyTuple]._2)
  catch case _: ClassCastException => println("CCE3")
