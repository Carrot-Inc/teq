// After Scala 3's tests/pos/packageobjs.scala and tests/pos/i239-packageObj.scala: a package
// object with parents, whose members and inherited members are the package's.
package shop

package object price extends PriceInstances:
  val tax: Double = 0.25
  def withTax(p: Double): Double = p * (1 + tax)
  def taxed: String = label(withTax(10.0).toString)
