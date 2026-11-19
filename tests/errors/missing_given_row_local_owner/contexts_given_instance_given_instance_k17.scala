trait G17
given gi17: G17 with {
  val r17 = try 1 catch { case _: Throwable => { def call(using need: Numeric[String]): Int = 1; call } }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_given_instance_given_instance_k17.scala:3:102: error: No given instance of type Numeric[String] was found for parameter need of method call in object gi17
