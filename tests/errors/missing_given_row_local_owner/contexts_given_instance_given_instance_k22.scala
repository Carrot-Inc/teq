trait G22
given gi22: G22 with {
  val r22 = if (true) { def call(using need: Numeric[String]): Int = 1; call } else 0
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_given_instance_given_instance_k22.scala:3:77: error: No given instance of type Numeric[String] was found for parameter need of method call in object gi22
