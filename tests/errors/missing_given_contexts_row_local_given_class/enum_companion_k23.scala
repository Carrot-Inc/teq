enum E23 { case A23 }
object E23 {
  val r23 = s"${ ({ given call(using need: Numeric[String]): Serializable with {}; call.hashCode }) }"
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: enum_companion_k23.scala:3:88: error: No given instance of type Numeric[String] was found for parameter need of given instance call in object E23
