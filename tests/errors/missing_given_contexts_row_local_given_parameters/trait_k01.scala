trait T01 {
  val r01 = { given call(using need: Numeric[String]): Int = 1; call }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: trait_k01.scala:2:69: error: No given instance of type Numeric[String] was found for parameter need of given instance call in trait T01
