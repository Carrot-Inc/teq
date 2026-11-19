object O30 { def call[T: Numeric as o](t: T): Int = 1 }

val result30 = O30.call("x")
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c30.scala:3:29: error: No given instance of type Numeric[String] was found for parameter o of method call in object O30
