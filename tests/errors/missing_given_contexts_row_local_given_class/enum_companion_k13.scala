enum E13 { case A13 }
object E13 {
  val r13 = for (i <- List(1) if { given call(using need: Numeric[String]): Serializable with {}; call.hashCode } > 0) yield i
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: enum_companion_k13.scala:3:103: error: No given instance of type Numeric[String] was found for parameter need of given instance call
