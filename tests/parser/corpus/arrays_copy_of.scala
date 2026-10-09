import java.util.Arrays

// `java.util.Arrays.copyOf` and `copyOfRange` keep the array's kind and pad with its zero, over
// the JVM arrays of link mode as over the lean std's, whose primitive overloads pad an empty
// array with its element type's zero and whose reference one pads with null.
@main def run(): Unit =
  val bytes = Array[Byte](71, 73, 70, 56)
  println(Arrays.copyOf(bytes, 2).mkString(","))
  println(Arrays.copyOf(bytes, 6).mkString(","))
  println(Arrays.copyOfRange(bytes, 1, 3).mkString(","))
  val words = Array("a", "b", "c")
  println(Arrays.copyOf(words, 4).mkString(","))
  println(Arrays.copyOfRange(words, 2, 3).mkString(","))
  val ints = Arrays.copyOf(Array(1, 2, 3), 5)
  println(ints.sum + " " + ints.length)
  for (from, to) <- List((2, 2), (-1, 0), (1, 0), (1, 3)) do
    try println(Arrays.copyOfRange(Array(1), from, to).mkString(","))
    catch case e: Exception => println(e.getClass.getName)
  println(Arrays.copyOf(Array.empty[Int], 2).mkString(","))
  println(Arrays.copyOf(Array.empty[Boolean], 1).mkString(","))
  println(Arrays.copyOfRange(Array.empty[Long], 0, 1).mkString(","))
  println(Arrays.copyOf(Array[AnyRef](Integer.valueOf(1)), 2).mkString(","))
