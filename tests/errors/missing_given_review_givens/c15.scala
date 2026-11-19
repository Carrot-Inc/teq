object O15 { given (using need: Numeric[String]): String = "x" }

val result15 = O15.given_String
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c15.scala:3:32: error: No given instance of type Numeric[String] was found for parameter need of given instance given_String in object O15
