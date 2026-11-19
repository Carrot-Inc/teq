class C08(using need: Numeric[String])

val result08 = new C08
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c08.scala:3:23: error: No given instance of type Numeric[String] was found for parameter need of constructor C08 in class C08
// teq: --std=scala-library
// expect: 5 errors found
