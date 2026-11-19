object O24 { inline def call(using need: Numeric[String]): Int = 1; inline def outer: Int = call }

val result24 = O24.outer
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c24.scala:1:97: error: No given instance of type Numeric[String] was found for parameter need of method call in object O24
