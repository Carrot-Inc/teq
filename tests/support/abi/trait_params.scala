// A trait's parameters in the class that mixes the trait in (dotty's `Mixin`, `traitInits`): the trait's abstract
// accessors, a `val`'s by its name (`using$rate`, a written `using$1` too), a private parameter's and a using
// parameter's expanded (`trait_params$T$$p`, a written `using$2`'s `trait_params$Spelled$$using$2`), an anonymous
// using parameter's `x$1`, `x$2` after a named one, a `var`'s setter beside its getter; the class's field, getter and setter, none for a
// parameter the class overrides; a default's getter on the trait's companion with its static
// forwarder.
// abi: classes v trait_params$T$$p w w_$eq trait_params$Ev$$ord trait_params$Anon$$x$1 trait_params$Tagged$$x$2 named $lessinit$greater$default$1 $lessinit$greater$default$3 o using$rate using$1 trait_params$Spelled$$using$2
package trait_params
trait T(val v: Int, p: Int, var w: Int = 3):
  def sum: Int = v + p + w
trait Ev[A](using ord: Ordering[A]):
  def max(a: A, b: A): A = ord.max(a, b)
trait Anon[A](using Ordering[A]):
  def min(a: A, b: A): A = summon[Ordering[A]].min(a, b)
trait Tagged(val tag: String)(using Int):
  def answer: Int = summon[Int]
trait Named(val named: String = "n")
trait Over(val o: Int)
trait Rate(val `using$rate`: Int)
trait Spelled(using val `using$1`: Int, `using$2`: String)
class C extends T(1, 2) with Ev[Int] with Anon[Int] with Named() with Tagged("t")(using 4)
class R extends Rate(7)
class S extends Spelled(using 1, "s")
class D extends Over(4):
  override val o: Int = 5
