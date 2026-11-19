enum E45 { case C(x: Int)(using need: Numeric[String]) }

val result45 = E45.C.apply(1)
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c45.scala:3:30: error: No given instance of type Numeric[String] was found for parameter need of method apply in object C
