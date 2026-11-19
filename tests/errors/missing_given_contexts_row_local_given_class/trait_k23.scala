trait T23 {
  val r23 = s"${ ({ given call(using need: Numeric[String]): Serializable with {}; call.hashCode }) }"
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: trait_k23.scala:2:88: error: No given instance of type Numeric[String] was found for parameter need of given instance call in trait T23
