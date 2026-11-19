trait T07 {
  val r07 = { { { given call(using need: Numeric[String]): Serializable with {}; call.hashCode } } }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: trait_k07.scala:2:86: error: No given instance of type Numeric[String] was found for parameter need of given instance call in trait T07
