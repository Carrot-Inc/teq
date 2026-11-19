def m08(x: Int = { given call(using need: Numeric[String]): Int = 1; call }): Int = x
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: empty_package_k08.scala:1:74: error: No given instance of type Numeric[String] was found for parameter need of given instance call
