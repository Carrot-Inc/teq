enum E07 { case A07 }
object E07 {
  val r07 = { { { def call(using need: Numeric[String]): Int = 1; call } } }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_enum_companion_enum_companion_k07.scala:3:71: error: No given instance of type Numeric[String] was found for parameter need of method call in object E07
