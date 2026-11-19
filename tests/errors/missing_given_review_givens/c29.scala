object O29 { def call[T: {Numeric, Ordering}](t: T): Int = 1 }
val result29 = O29.call("x")
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c29.scala:2:29: error: No given instance of type Numeric[String] was found for a context parameter of method call in object O29
