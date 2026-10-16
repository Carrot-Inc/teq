// A trait's polymorphic clone, which overrides no `Object.clone()` (their types do not match before erasure):
// a class mixing it in has its forwarder and no bridge of `Object`'s, so that `Object.clone` of a subclass is
// still `Object`'s.
package polyclone

trait G:
  def clone[A](): java.util.List[A] = null
