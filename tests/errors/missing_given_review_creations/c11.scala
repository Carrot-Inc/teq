case class C11(x: Int)(using need: Numeric[String])

val result11 = C11(1)
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c11.scala:3:22: error: No given instance of type Numeric[String] was found for parameter need of method apply in object C11
