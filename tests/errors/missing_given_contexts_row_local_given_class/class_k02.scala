class C02() {
  val (r02, s02) = ({ given call(using need: Numeric[String]): Serializable with {}; call.hashCode }, 2)
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: class_k02.scala:2:90: error: No given instance of type Numeric[String] was found for parameter need of given instance call in class C02
