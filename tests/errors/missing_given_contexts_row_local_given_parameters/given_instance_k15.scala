trait G15
given gi15: G15 with {
  val r15 = { while (false) { { given call(using need: Numeric[String]): Int = 1; call } }; 1 }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: given_instance_k15.scala:3:87: error: No given instance of type Numeric[String] was found for parameter need of given instance call in object gi15
