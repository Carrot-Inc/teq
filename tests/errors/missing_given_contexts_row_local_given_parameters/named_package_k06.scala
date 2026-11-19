package p06
lazy val r06 = { given call(using need: Numeric[String]): Int = 1; call }
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: named_package_k06.scala:2:72: error: No given instance of type Numeric[String] was found for parameter need of given instance call
