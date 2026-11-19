enum E14 { case A14 }
object E14 {
  val r14 = for (i <- List(1)) yield { given call(using need: Numeric[String]): Serializable with {}; call.hashCode }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: enum_companion_k14.scala:3:107: error: No given instance of type Numeric[String] was found for parameter need of given instance call
