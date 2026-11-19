package p12
val r12 = for (i <- List({ def call(using need: Numeric[String]): Int = 1; call })) yield i
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_named_package_named_package_k12.scala:2:80: error: No given instance of type Numeric[String] was found for parameter need of method call in package p12
