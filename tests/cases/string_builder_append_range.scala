// `StringBuilder.append(CharSequence, start, end)`: the characters of the range, a `null` as
// `"null"`, a range outside the text an exception.
@main def run(): Unit =
  val sb = new java.lang.StringBuilder("<")
  sb.append("abcdef", 1, 4).append(new java.lang.StringBuilder("xyz"), 0, 2)
  println(sb.toString)
  println(new java.lang.StringBuilder().append(null: CharSequence, 0, 2).toString)
  try new java.lang.StringBuilder().append("ab", 1, 5)
  catch case e: IndexOutOfBoundsException => println("out of range")
