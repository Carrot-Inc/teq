package rowna

// The members a downstream reaches through a subclass, a bounded opaque type, an inline body, a
// trait overriding Object's `clone` and an object inheriting a class's members: a member reference
// names the qualifier's class (dotty's `BCodeBodyBuilder.genApply`), which the downstream's split build
// reads from these products as its whole build reads it from the sources, the same bytes.
abstract class Root:
  final def deep(): Int = 11

abstract class Base extends Root:
  var field: Int = 5
  final def method(): Int = 7
  def withDefault(x: Int = 3): Int = x

class PublicChild extends Base
class GrandChild extends PublicChild

trait CloneBase:
  override def clone(): Object = this
trait CloneChild extends CloneBase
class Cloned extends CloneChild

object Library:
  opaque type Wrapped <: Base = PublicChild
  def wrap(x: PublicChild): Wrapped = x
  inline def inlineMethod(x: PublicChild): Int = x.method()
  inline def inlineBound[T <: PublicChild](x: T): Int = x.deep()
  def child(): PublicChild = new PublicChild()

object Inherited extends PublicChild
