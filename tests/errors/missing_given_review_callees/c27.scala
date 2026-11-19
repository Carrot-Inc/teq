object O27 { def call(i: Int)(s: String)(using need: Numeric[String]): Int = i }

val result27 = O27.call(1)("x")
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c27.scala:3:32: error: No given instance of type Numeric[String] was found for parameter need of method call in object O27
