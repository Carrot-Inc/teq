class C21() {
  val r21 = { given Int = 1; { given call(using need: Numeric[String]): Int = 1; call } }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: class_k21.scala:2:86: error: No given instance of type Numeric[String] was found for parameter need of given instance call in class C21
