enum E03 { case A03 }
object E03 {
  val List(r03) = List({ def call(using need: Numeric[String]): Int = 1; call }): @unchecked
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_enum_companion_enum_companion_k03.scala:3:78: error: No given instance of type Numeric[String] was found for parameter need of method call in object E03
