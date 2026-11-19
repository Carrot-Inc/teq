enum E01 { case A01 }
object E01 {
  val r01 = { given call(using need: Numeric[String]): Serializable with {}; call.hashCode }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: enum_companion_k01.scala:3:82: error: No given instance of type Numeric[String] was found for parameter need of given instance call in object E01
