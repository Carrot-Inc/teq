object O14 { given make(using need: Numeric[String]): String = "x" }

val result14 = O14.make
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c14.scala:3:24: error: No given instance of type Numeric[String] was found for parameter need of given instance make in object O14
// teq: --std=scala-library
// expect: 8 errors found
