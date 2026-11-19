val r01 = { def call(using need: Numeric[String]): Int = 1; call }
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: empty_package_k01.scala:1:65: error: No given instance of type Numeric[String] was found for parameter need of method call
// teq: --std=scala-library
// expect: 17 errors found
