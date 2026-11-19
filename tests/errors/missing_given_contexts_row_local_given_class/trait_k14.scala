trait T14 {
  val r14 = for (i <- List(1)) yield { given call(using need: Numeric[String]): Serializable with {}; call.hashCode }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: trait_k14.scala:2:107: error: No given instance of type Numeric[String] was found for parameter need of given instance call
