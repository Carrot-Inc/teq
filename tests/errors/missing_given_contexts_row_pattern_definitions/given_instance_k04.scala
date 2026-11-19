trait G04
given gi04: G04 with {
  val (r04: Int) = { def call(using need: Numeric[String]): Int = 1; call }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: given_instance_k04.scala:3:74: error: No given instance of type Numeric[String] was found for parameter need of method call in object gi04
