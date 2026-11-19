trait T36 { def call(using need: Numeric[String]): Int }; class C extends T36 { def call(using need: Numeric[String]): Int = 1 }

val parent36: T36 = new C
val result36 = parent36.call
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c36.scala:4:29: error: No given instance of type Numeric[String] was found for parameter need of method call in trait T36
