def use46 = { class Local { def call(using need: Numeric[String]): Int = 1 }; (new Local).call }
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c46.scala:1:95: error: No given instance of type Numeric[String] was found for parameter need of method call in class Local
