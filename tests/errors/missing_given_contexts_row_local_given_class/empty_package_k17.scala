val r17 = try 1 catch { case _: Throwable => { given call(using need: Numeric[String]): Serializable with {}; call.hashCode } }
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: empty_package_k17.scala:1:115: error: No given instance of type Numeric[String] was found for parameter need of given instance call
