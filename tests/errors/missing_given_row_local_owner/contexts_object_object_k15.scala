object O15 {
  val r15 = { while (false) { { def call(using need: Numeric[String]): Int = 1; call } }; 1 }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_object_object_k15.scala:2:85: error: No given instance of type Numeric[String] was found for parameter need of method call in object O15
