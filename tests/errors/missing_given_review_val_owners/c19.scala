val result19 = (i: Int) => { def call(using need: Numeric[String]): Int = i; call }
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c19.scala:1:82: error: No given instance of type Numeric[String] was found for parameter need of method call
// teq: --std=scala-library
// expect: 2 errors found
