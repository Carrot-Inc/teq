trait G14
given gi14: G14 with {
  val r14 = for (i <- List(1)) yield { given call(using need: Numeric[String]): Int = 1; call }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: given_instance_k14.scala:3:94: error: No given instance of type Numeric[String] was found for parameter need of given instance call
