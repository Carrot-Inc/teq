object O02 { def call(using need: Numeric[String]): Int = 1 }

val result02 = O02.call
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c02.scala:3:24: error: No given instance of type Numeric[String] was found for parameter need of method call in object O02
