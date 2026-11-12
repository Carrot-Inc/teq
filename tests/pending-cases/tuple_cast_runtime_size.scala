// scalac 3.8.4 prints the .expected. A Tuple3 passes the cast
// to `Int *: Int *: EmptyTuple`, which scalac erases to Product
// (tupleArity is -1 for a chain ending in the EmptyTuple alias,
// TypeErasure.scala:82-95), and `size` is `runtime.Tuples.size` of the
// value (Tuple.scala:66-67), 3. teq's interpreter, Scala.js and JVM
// builds are to print 3 too. scalac's `._2` throws ClassCastException.
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
