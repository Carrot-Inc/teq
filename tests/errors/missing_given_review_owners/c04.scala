package probe04
package object po { def call(using need: Numeric[String]): Int = 1 }

val result04 = po.call
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c04.scala:4:23: error: No given instance of type Numeric[String] was found for parameter need of method call in package probe04.po
