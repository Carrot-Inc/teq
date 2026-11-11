object B { def f(): Unit = { 1; () } }
// expect: B.scala:1:30: warning: A pure expression does nothing in statement position
