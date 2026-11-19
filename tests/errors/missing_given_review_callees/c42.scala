val result42 = implicitly[Numeric[String]]
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c42.scala:1:43: error: No given instance of type Numeric[String] was found for parameter e of method implicitly in object Predef
