class C09(val x: Int) { def this(s: String)(using need: Numeric[String]) = this(s.length) }

val result09 = new C09("x")
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c09.scala:3:28: error: No given instance of type Numeric[String] was found for parameter need of constructor C09 in class C09
