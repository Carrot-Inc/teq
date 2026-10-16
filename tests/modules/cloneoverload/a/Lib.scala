// A trait's `clone` override beside an overload of the name (`clone(Int)`): the class mixing it in takes
// the forwarder of the declaration that overrides `Object`'s, each alternative a declaration of its own
// (`Mixin.mixinForwarders` asks `needsMixinForwarder` of each), whether the build reads the trait from its
// source or from these products; without it the JVM selects `Object`'s protected `clone`. The overload,
// which competes with no class's method, has none.
package cloneoverload

trait C:
  override def clone(): Object = "overload"
  def clone(x: Int): String = "int " + x
trait D extends C
