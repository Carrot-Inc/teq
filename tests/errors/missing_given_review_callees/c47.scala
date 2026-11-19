import scala.reflect.Selectable.reflectiveSelectable
def use47(x: { def call(using need: Numeric[String]): Int }) = x.call
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c47.scala:2:70: error: No given instance of type Numeric[String] was found for parameter need of method call
