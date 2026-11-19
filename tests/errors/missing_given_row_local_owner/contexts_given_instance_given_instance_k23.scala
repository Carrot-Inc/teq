trait G23
given gi23: G23 with {
  val r23 = s"${ ({ def call(using need: Numeric[String]): Int = 1; call }) }"
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_given_instance_given_instance_k23.scala:3:73: error: No given instance of type Numeric[String] was found for parameter need of method call in object gi23
