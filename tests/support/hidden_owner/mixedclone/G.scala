// Two traits' clones, an override of `Object`'s and a polymorphic one, two members `Mixin.mixinForwarders` asks
// `needsMixinForwarder` of each on its own: a class mixing both in has a forwarder of C's, which competes with
// `Object`'s, so that `(new E: C).clone()` is C's and not `Object`'s protected one.
package mixedclone

trait C:
  override def clone(): Object = "cloned"

trait G:
  def clone[A](): java.util.List[A] = null
