class C16() {
  val r16 = try { given call(using need: Numeric[String]): Int = 1; call } catch { case _: Throwable => 1 }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: class_k16.scala:2:73: error: No given instance of type Numeric[String] was found for parameter need of given instance call in class C16
