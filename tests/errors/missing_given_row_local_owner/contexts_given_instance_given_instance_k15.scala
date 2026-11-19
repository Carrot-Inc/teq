trait G15
given gi15: G15 with {
  val r15 = { while (false) { { def call(using need: Numeric[String]): Int = 1; call } }; 1 }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_given_instance_given_instance_k15.scala:3:85: error: No given instance of type Numeric[String] was found for parameter need of method call in object gi15
