// A trait's call through `super` of an inline method with an inline parameter's default, in the upstream and in a
// downstream trait: the method and its getter, an inline method of its own (Desugar.scala:429), are expanded where
// they are called, and have no super accessor (`SuperAccessors.transformSuperSelect`'s `needAccessor`), so neither
// build gives the classes mixing the traits in an accessor of either, the pickles select them on `super` itself, and
// scalac 3.8.4 compiles the downstream over teq's products, its classes owing no implementation of an accessor.
package tsid

trait T:
  def tk: Int = 1
  inline def tm(inline x: Int = tk + 5): Int = x * 2

trait T2 extends T:
  override def tk: Int = 10
  def f: Int = super.tm()

class C2 extends T2
