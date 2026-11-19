trait G10
given gi10: G10 with {
  val r10 = (i: Int) => { given call(using need: Numeric[String]): Serializable with {}; call.hashCode }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: given_instance_k10.scala:3:94: error: No given instance of type Numeric[String] was found for parameter need of given instance call
