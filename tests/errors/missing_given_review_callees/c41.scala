val result41 = summon[Numeric[String]]
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c41.scala:1:39: error: No given instance of type Numeric[String] was found for parameter x of method summon in object Predef
