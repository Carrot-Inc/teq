class `Odd Class` { def `odd method`(using need: Numeric[String]): Int = 1 }

val result48 = (new `Odd Class`).`odd method`
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c48.scala:3:46: error: No given instance of type Numeric[String] was found for parameter need of method odd method in class Odd Class
