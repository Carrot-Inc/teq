trait T03 { def call(using need: Numeric[String]): Int = 1 }

def use03(t: T03) = t.call
// scalac 3.8.4 gives each line whole at one column less (the code review's owner programs, rewritten).
// expect: c03.scala:3:27: error: No given instance of type Numeric[String] was found for parameter need of method call in trait T03
