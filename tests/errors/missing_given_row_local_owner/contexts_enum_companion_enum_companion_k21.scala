enum E21 { case A21 }
object E21 {
  val r21 = { given Int = 1; { def call(using need: Numeric[String]): Int = 1; call } }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_enum_companion_enum_companion_k21.scala:3:84: error: No given instance of type Numeric[String] was found for parameter need of method call in object E21
