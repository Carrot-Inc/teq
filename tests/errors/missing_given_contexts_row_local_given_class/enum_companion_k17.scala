enum E17 { case A17 }
object E17 {
  val r17 = try 1 catch { case _: Throwable => { given call(using need: Numeric[String]): Serializable with {}; call.hashCode } }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: enum_companion_k17.scala:3:117: error: No given instance of type Numeric[String] was found for parameter need of given instance call in object E17
