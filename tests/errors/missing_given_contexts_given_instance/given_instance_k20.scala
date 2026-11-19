trait G20
given gi20: G20 with {
  { def call(using need: Numeric[String]): Int = 1; call }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: given_instance_k20.scala:3:57: error: No given instance of type Numeric[String] was found for parameter need of method call
