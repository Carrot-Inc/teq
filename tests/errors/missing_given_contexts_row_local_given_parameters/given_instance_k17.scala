trait G17
given gi17: G17 with {
  val r17 = try 1 catch { case _: Throwable => { given call(using need: Numeric[String]): Int = 1; call } }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: given_instance_k17.scala:3:104: error: No given instance of type Numeric[String] was found for parameter need of given instance call in object gi17
