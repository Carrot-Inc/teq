//> using options -Wconf:any:e
object B { def g(): Unit = { 2; () } }
// expect: B.scala:2:30: error: A pure expression does nothing in statement position
