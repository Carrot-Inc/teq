object O28 { def call[T: Numeric](t: T): Int = 1 }

val result28 = O28.call("x")
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c28.scala:3:29: error: No given instance of type Numeric[String] was found for a context parameter of method call in object O28
