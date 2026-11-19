def use31(f: Numeric[String] ?=> Int): Int = f
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c31.scala:1:47: error: No given instance of type Numeric[String] was found for parameter of (Numeric[String]) ?=> Int
// teq: --std=scala-library
// expect: 3 errors found
