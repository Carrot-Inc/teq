object O23 { inline def call(using need: Numeric[String]): Int = 1 }

val result23 = O23.call
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c23.scala:3:24: error: No given instance of type Numeric[String] was found for parameter need of method call in object O23
// teq: --std=scala-library
// expect: 12 errors found
