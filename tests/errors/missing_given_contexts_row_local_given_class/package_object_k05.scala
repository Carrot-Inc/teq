package q05
package object po05 {
  var r05 = { given call(using need: Numeric[String]): Serializable with {}; call.hashCode }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: package_object_k05.scala:3:82: error: No given instance of type Numeric[String] was found for parameter need of given instance call in package q05.po05
