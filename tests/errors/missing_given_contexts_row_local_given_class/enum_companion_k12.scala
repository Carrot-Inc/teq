enum E12 { case A12 }
object E12 {
  val r12 = for (i <- List({ given call(using need: Numeric[String]): Serializable with {}; call.hashCode })) yield i
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: enum_companion_k12.scala:3:97: error: No given instance of type Numeric[String] was found for parameter need of given instance call in object E12
