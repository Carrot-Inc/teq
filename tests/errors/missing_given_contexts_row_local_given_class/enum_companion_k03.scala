enum E03 { case A03 }
object E03 {
  val List(r03) = List({ given call(using need: Numeric[String]): Serializable with {}; call.hashCode }): @unchecked
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: enum_companion_k03.scala:3:93: error: No given instance of type Numeric[String] was found for parameter need of given instance call in object E03
