enum E16 { case A16 }
object E16 {
  val r16 = try { given call(using need: Numeric[String]): Serializable with {}; call.hashCode } catch { case _: Throwable => 1 }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: enum_companion_k16.scala:3:86: error: No given instance of type Numeric[String] was found for parameter need of given instance call in object E16
