// A trait's super accessors (dotty's `SuperAccessors`, `ResolveSuper`): the trait declares each with the member as
// the trait sees it, `id(String): String` for `Parent[String]`'s `id(A): A` and `narrow[B <: String]` for its
// `narrow[B <: A]`, `String` through a chain of bounds in either order (`chain[B <: C, C <: A]`), the parent's own
// erasure where the trait is generic too, and the class that mixes the trait in defines it, calling the next
// definition.
// abi: Stack#super_accessors$Stack$$super$id Stack#super_accessors$Stack$$super$pair Stack#super_accessors$Stack$$super$narrow Stack#super_accessors$Stack$$super$chain Stack#super_accessors$Stack$$super$back Child#super_accessors$Stack$$super$id Child#super_accessors$Stack$$super$pair Child#super_accessors$Stack$$super$narrow Child#super_accessors$Stack$$super$chain Child#super_accessors$Stack$$super$back Twice#super_accessors$Twice$$super$id
package super_accessors
trait Parent[A]:
  def id(x: A): A
  def pair[B](x: A, y: B): (A, B)
  def narrow[B <: A](x: B): B
  def chain[B <: C, C <: A](x: B): B
  def back[C <: A, B <: C](x: B): C
trait Stack extends Parent[String]:
  abstract override def id(x: String): String = super.id(x) + "!"
  abstract override def pair[B](x: String, y: B): (String, B) = super.pair(x + "?", y)
  abstract override def narrow[B <: String](x: B): B = super.narrow[B](x)
  abstract override def chain[B <: C, C <: String](x: B): B = super.chain[B, C](x)
  abstract override def back[C <: String, B <: C](x: B): C = super.back[C, B](x)
class Base extends Parent[String]:
  def id(x: String): String = x
  def pair[B](x: String, y: B): (String, B) = (x, y)
  def narrow[B <: String](x: B): B = x
  def chain[B <: C, C <: String](x: B): B = x
  def back[C <: String, B <: C](x: B): C = x
class Child extends Base with Stack
trait Twice[A] extends Parent[A]:
  abstract override def id(x: A): A = super.id(super.id(x))
