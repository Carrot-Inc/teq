class C07() {
  val r07 = { { { given call(using need: Numeric[String]): Int = 1; call } } }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: class_k07.scala:2:73: error: No given instance of type Numeric[String] was found for parameter need of given instance call in class C07
