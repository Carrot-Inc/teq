opaque type Token37 = Int
object Token37 { def call(using need: Numeric[String]): Token37 = 1 }

val result37 = Token37.call
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c37.scala:4:28: error: No given instance of type Numeric[String] was found for parameter need of method call in object Token37
