def call07(using need: Numeric[String]): Int = 1

val result07 = call07
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c07.scala:3:22: error: No given instance of type Numeric[String] was found for parameter need of method call07
