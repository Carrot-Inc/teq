trait G16
given gi16: G16 with {
  val r16 = try { given call(using need: Numeric[String]): Serializable with {}; call.hashCode } catch { case _: Throwable => 1 }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: given_instance_k16.scala:3:86: error: No given instance of type Numeric[String] was found for parameter need of given instance call in object gi16
