object O14 {
  val r14 = for (i <- List(1)) yield { given call(using need: Numeric[String]): Int = 1; call }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: object_k14.scala:2:94: error: No given instance of type Numeric[String] was found for parameter need of given instance call
