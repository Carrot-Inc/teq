object O13 {
  val r13 = for (i <- List(1) if { given call(using need: Numeric[String]): Int = 1; call } > 0) yield i
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: object_k13.scala:2:90: error: No given instance of type Numeric[String] was found for parameter need of given instance call
