val r23 = s"${ ({ given call(using need: Numeric[String]): Int = 1; call }) }"
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: empty_package_k23.scala:1:73: error: No given instance of type Numeric[String] was found for parameter need of given instance call
