package outer06.inner06
def call06(using need: Numeric[String]): Int = 1

val result06 = call06
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c06.scala:4:22: error: No given instance of type Numeric[String] was found for parameter need of method call06 in package outer06.inner06
