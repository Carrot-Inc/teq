enum E15 { case A15 }
object E15 {
  val r15 = { while (false) { { given call(using need: Numeric[String]): Serializable with {}; call.hashCode } }; 1 }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: enum_companion_k15.scala:3:100: error: No given instance of type Numeric[String] was found for parameter need of given instance call in object E15
