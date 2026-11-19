trait T16 {
  val r16 = try { def call(using need: Numeric[String]): Int = 1; call } catch { case _: Throwable => 1 }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_trait_trait_k16.scala:2:71: error: No given instance of type Numeric[String] was found for parameter need of method call in trait T16
