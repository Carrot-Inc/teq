trait X44 { def call(using need: Numeric[String]): Int = 1 }; object O { given gx: X44 with {} }

val result44 = O.gx.call
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c44.scala:3:25: error: No given instance of type Numeric[String] was found for parameter need of method call in trait X44
