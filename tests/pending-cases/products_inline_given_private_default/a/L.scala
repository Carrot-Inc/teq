// The library, built into products: an inline given and an inline method whose inline using parameter's
// default reads a private member of the class.
package lib7
class C(private val k: Int):
  inline given g(using inline x: Int = k + 1): String = s"g$x"
  inline def d(using inline x: Int = k + 2): String = s"d$x"
