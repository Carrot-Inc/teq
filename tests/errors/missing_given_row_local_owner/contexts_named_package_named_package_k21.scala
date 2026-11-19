package p21
val r21 = { given Int = 1; { def call(using need: Numeric[String]): Int = 1; call } }
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_named_package_named_package_k21.scala:2:82: error: No given instance of type Numeric[String] was found for parameter need of method call in package p21
