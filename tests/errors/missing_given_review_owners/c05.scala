package probe05
def call05(using need: Numeric[String]): Int = 1

val result05 = call05
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c05.scala:4:22: error: No given instance of type Numeric[String] was found for parameter need of method call05 in package probe05
