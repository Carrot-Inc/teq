trait G22
given gi22: G22 with {
  val r22 = if (true) { given call(using need: Numeric[String]): Int = 1; call } else 0
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: given_instance_k22.scala:3:79: error: No given instance of type Numeric[String] was found for parameter need of given instance call in object gi22
