// A concatenation's operand that is a conditional of two string constants, typed by the union of
// their constant types, is a String to the emitter (dotty's `widenUnion`: `" to " | " until "` is a
// `String`): it is written as it is, with no conversion around it and no closure hoisting the
// concatenation. The std's `Range.toString` is one, a conditional of two numeric constants another.
@main def run(): Unit =
  println((1 to 3).toString)
  println((1 until 7 by 2).toString)
  for n <- List(1, 2) do
    val flag = n == 1
    println("item" + (if flag then "" else "s") + " " + n)
    println("code " + (if flag then 4 else 5))
