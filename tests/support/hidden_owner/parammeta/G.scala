// A scalac-built trait's annotated parameters: an overload of `clone` that overrides nothing of `Object`'s, which
// the class mixing it in does not forward, so that reflection finds the trait's method and its annotation; and
// an override of `equals`, whose forwarder carries the parameter's annotation (`MixinOps.mkForwarderSym`).
package pm

trait C:
  def clone(@Tag("arg") x: String): String = x
  override def equals(@Tag("eq") x: Any): Boolean = x.isInstanceOf[C]
