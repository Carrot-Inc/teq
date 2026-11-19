class C01 { def call(using need: Numeric[String]): Int = 1 }

val result01 = (new C01).call
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c01.scala:3:30: error: No given instance of type Numeric[String] was found for parameter need of method call in class C01
// teq: --std=scala-library
// expect: 13 errors found
