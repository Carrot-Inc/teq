trait G03
given gi03: G03 with {
  val List(r03) = List({ given call(using need: Numeric[String]): Serializable with {}; call.hashCode }): @unchecked
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: given_instance_k03.scala:3:93: error: No given instance of type Numeric[String] was found for parameter need of given instance call in object gi03
