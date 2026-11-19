def use17 = { def call(using need: Numeric[String]): Int = 1; call }
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c17.scala:1:67: error: No given instance of type Numeric[String] was found for parameter need of method call
