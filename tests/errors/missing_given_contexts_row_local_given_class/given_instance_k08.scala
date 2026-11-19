trait G08
given gi08: G08 with {
  def m08(x: Int = { given call(using need: Numeric[String]): Serializable with {}; call.hashCode }): Int = x
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: given_instance_k08.scala:3:89: error: No given instance of type Numeric[String] was found for parameter need of given instance call
