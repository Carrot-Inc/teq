object O08 {
  def m08(x: Int = { given call(using need: Numeric[String]): Serializable with {}; call.hashCode }): Int = x
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: object_k08.scala:2:89: error: No given instance of type Numeric[String] was found for parameter need of given instance call
