enum E22 { case A22 }
object E22 {
  val r22 = if (true) { def call(using need: Numeric[String]): Int = 1; call } else 0
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_enum_companion_enum_companion_k22.scala:3:77: error: No given instance of type Numeric[String] was found for parameter need of method call in object E22
