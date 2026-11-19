object O33 { def call(using need: Numeric[String] = summon[Numeric[String]]): Int = 1 }

val result33 = O33.call
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c33.scala:1:76: error: No given instance of type Numeric[String] was found for parameter x of method summon in object Predef
