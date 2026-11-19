class C15() {
  val r15 = { while (false) { { def call(using need: Numeric[String]): Int = 1; call } }; 1 }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_class_class_k15.scala:2:85: error: No given instance of type Numeric[String] was found for parameter need of method call in class C15
