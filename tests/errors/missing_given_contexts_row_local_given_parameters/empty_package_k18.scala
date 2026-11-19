val r18 = s"${ given call(using need: Numeric[String]): Int = 1; call }"
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: empty_package_k18.scala:1:70: error: No given instance of type Numeric[String] was found for parameter need of given instance call
