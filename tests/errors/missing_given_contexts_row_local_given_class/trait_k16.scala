trait T16 {
  val r16 = try { given call(using need: Numeric[String]): Serializable with {}; call.hashCode } catch { case _: Throwable => 1 }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: trait_k16.scala:2:86: error: No given instance of type Numeric[String] was found for parameter need of given instance call in trait T16
