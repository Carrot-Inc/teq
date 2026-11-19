trait X16
object O16 { given make(using need: Numeric[String]): X16 with {} }

val result16 = O16.make
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c16.scala:4:24: error: No given instance of type Numeric[String] was found for parameter need of given instance make in object O16
