package cta

// A constructor's declared access is written whole, its qualifier included, and a constructor its
// boundary lets another module call is published: a downstream inside `cta` creates through each.
class P3 private[cta] (val x: Int)
class P4 protected[cta] (val x: Int)
class Prot protected (val x: Int)

class S(val x: Int):
  private[cta] def this() = this(7)
  protected[cta] def this(s: String) = this(s.length)
  protected def this(a: Int, b: Int) = this(a + b)

// The more specific alternative is hidden outside `cta`: the primary takes the argument there.
class Hidden(val x: Any):
  private[cta] def this(x: String) = this("secondary".asInstanceOf[Any])

// Its `apply`, `copy` and `copy`'s default getters carry the constructor's access, as scalac reads them.
case class K private[cta] (x: Int)

object InPkg:
  def own(): Int = new P3(1).x + new S().x

// An anonymous class of an empty body through a protected constructor, in a body a downstream module reads
// from these products: typed here, where it stands, and not asked again there.
object Made:
  def anonymous(): Prot = new Prot(13) {}
