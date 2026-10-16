// A trait's overload of `clone` that overrides nothing of `Object`'s (`clone(x: Long, y: Int)`): no forwarder of
// it, as of every trait method that competes with no class's (`MixinOps.needsMixinForwarder`); the class's call
// of it reaches the trait's default, whose parameters take their slots by their widths, a `Long` two.
package clonewide

trait T:
  def clone(x: Long, y: Int): String = y.toString
