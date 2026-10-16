// A generic trait's `clone(x: A)` and a trait's override of it at String (`clone(x: String)`): neither overrides
// `Object`'s, so the class mixing in the second has no forwarder of either (`MixinOps.needsMixinForwarder`), and
// a call through the generic trait reaches the override, by the bridge the overriding trait has
// (`Bridges.addBridgeIfNeeded` pairs the two before erasure).
package clonespecialized

trait G[A]:
  def clone(x: A): AnyRef = "generic"

trait H extends G[String]:
  override def clone(x: String): String = "special"
