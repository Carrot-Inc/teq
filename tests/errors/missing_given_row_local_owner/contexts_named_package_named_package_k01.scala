package p01
val r01 = { def call(using need: Numeric[String]): Int = 1; call }
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_named_package_named_package_k01.scala:2:65: error: No given instance of type Numeric[String] was found for parameter need of method call in package p01
