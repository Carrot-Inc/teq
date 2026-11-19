trait G16
given gi16: G16 with {
  val r16 = try { def call(using need: Numeric[String]): Int = 1; call } catch { case _: Throwable => 1 }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_given_instance_given_instance_k16.scala:3:71: error: No given instance of type Numeric[String] was found for parameter need of method call in object gi16
