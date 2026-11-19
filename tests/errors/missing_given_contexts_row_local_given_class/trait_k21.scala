trait T21 {
  val r21 = { given Int = 1; { given call(using need: Numeric[String]): Serializable with {}; call.hashCode } }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: trait_k21.scala:2:99: error: No given instance of type Numeric[String] was found for parameter need of given instance call in trait T21
