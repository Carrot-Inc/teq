// A trait's `clone` that declares the exceptions it throws, deprecated and annotated: the forwarder a class
// mixing it in has carries each over (`MixinOps.mkForwarderSym`), as the trait's class file holds them.
package throwsclone

trait ThrowsClone:
  @throws[CloneNotSupportedException]
  @throws[java.io.IOException]
  @deprecated("use copy", "1")
  @java.lang.Deprecated
  override def clone(): Object = throw new CloneNotSupportedException("throws")
