object Origin38 { def call(using need: Numeric[String]): Int = 1 }; object Exporter { export Origin38.call }

val result38 = Exporter.call
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c38.scala:3:29: error: No given instance of type Numeric[String] was found for parameter need of method call in object Exporter
