trait G04
given gi04: G04 with {
  val (r04: Int) = { given call(using need: Numeric[String]): Serializable with {}; call.hashCode }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: given_instance_k04.scala:3:89: error: No given instance of type Numeric[String] was found for parameter need of given instance call in object gi04
