trait T03 {
  val List(r03) = List({ given call(using need: Numeric[String]): Serializable with {}; call.hashCode }): @unchecked
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: trait_k03.scala:2:93: error: No given instance of type Numeric[String] was found for parameter need of given instance call in trait T03
