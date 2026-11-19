trait G02
given gi02: G02 with {
  val (r02, s02) = ({ given call(using need: Numeric[String]): Serializable with {}; call.hashCode }, 2)
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: given_instance_k02.scala:3:90: error: No given instance of type Numeric[String] was found for parameter need of given instance call in object gi02
