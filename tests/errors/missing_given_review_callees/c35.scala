object O35 { def call(i: Int)(using need: Numeric[String]): Int = i; def call(s: String)(using n: String): String = s }

val result35 = O35.call(1)
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c35.scala:3:27: error: No given instance of type Numeric[String] was found for parameter need of method call in object O35
