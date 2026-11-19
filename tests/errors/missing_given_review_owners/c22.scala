class Outer22 { object O { class C { def call(using need: Numeric[String]): Int = 1 } } }

val outer22 = new Outer22
val result22 = (new outer22.O.C).call
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c22.scala:4:38: error: No given instance of type Numeric[String] was found for parameter need of method call in class C
