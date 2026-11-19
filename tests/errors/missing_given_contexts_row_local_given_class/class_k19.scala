class C19() {
  def this(s19: String) = { this(); { given call(using need: Numeric[String]): Serializable with {}; call.hashCode } }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: class_k19.scala:2:106: error: No given instance of type Numeric[String] was found for parameter need of given instance call
