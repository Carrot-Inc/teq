class C11() {
  val r11: PartialFunction[Int, Int] = { case i => { given call(using need: Numeric[String]): Serializable with {}; call.hashCode } }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: class_k11.scala:2:121: error: No given instance of type Numeric[String] was found for parameter need of given instance call
