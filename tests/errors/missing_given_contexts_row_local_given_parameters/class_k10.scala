class C10() {
  val r10 = (i: Int) => { given call(using need: Numeric[String]): Int = 1; call }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: class_k10.scala:2:81: error: No given instance of type Numeric[String] was found for parameter need of given instance call
