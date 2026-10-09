// The forwarders a class gets for the `toString`, `hashCode` and `equals` of its traits, as scalac's `Mixin`
// writes them (`MixinOps.needsMixinForwarder`): where the first of the linearisation to define one is a trait the
// superclass does not mix in already; none where a class defines it or the superclass forwards it.
// abi: C#toString C#hashCode C#equals D#toString D#hashCode D#equals E#toString E#hashCode E#equals F#toString F#hashCode F#equals G#toString G#hashCode G#equals
package forwarders
trait Named:
  override def toString: String = "named"
  override def hashCode: Int = 123
  override def equals(x: Any): Boolean = x.isInstanceOf[Named]
trait Last extends Named:
  override def toString: String = "last"
class B extends Named:
  override def toString: String = "b"
class C extends B
class D extends Last
class E extends B with Last
class F extends Named
class G extends F
