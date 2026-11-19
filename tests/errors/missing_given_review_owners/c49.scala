object `Odd.Object` { def `odd.method`(using need: Numeric[String]): Int = 1 }

val result49 = `Odd.Object`.`odd.method`
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c49.scala:3:41: error: No given instance of type Numeric[String] was found for parameter need of method odd.method in object Odd.Object
