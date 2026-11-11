// A derived instance over a type that contains itself and has no instance of its own, with the shapes of shapeless3
// and kittens written here: `OrElse` (an inline given over `summonFrom`), the instances of a product's or a sum's
// elements summoned inline over a mirror's element types (`mkInstances`), `Derived` with its `product` and
// `coproduct` givens and their by-name instances. `Element` holds a `List[Item]`, `Item` without an `Eq`: the list
// derives through its sum, whose `::` holds the list again, and the second request for the list's instance is tied
// to the first through the by-name instances the inline `mkInstances` answers (`given_knot_inline_candidate` is the
// same in a few lines). scalac 3.8.4 prints `true`, `false`, `false`.
import scala.compiletime.*
import scala.deriving.Mirror

trait Eq[A]:
  def eqv(x: A, y: A): Boolean
object Eq:
  given Eq[String] = _ == _

// shapeless3's OrElse: the instance there is, else the derived one
type OrElse[A, B] = A | B
object OrElse:
  inline given orElse[A, B]: OrElse[A, B] = summonFrom:
    case a: A => a
    case b: B => b

// kittens' Derived and its Or
final class Derived[A](val instance: A)
type Or[A] = OrElse[Eq[A], Derived[Eq[A]]]

// shapeless3's instances of a product's or a sum's elements, summoned inline, erased
final class Instances[F[_], T](val elems: () => List[Any])
inline def summonAll[F[_], T <: Tuple]: List[Any] = inline erasedValue[T] match
  case _: EmptyTuple => Nil
  case _: (h *: t) => summonInline[F[h]] :: summonAll[F, t]
object Instances:
  inline given mkInstances[F[_], T](using m: Mirror.Of[T]): Instances[F, T] =
    new Instances(() => summonAll[F, m.MirroredElemTypes])

object Derived:
  private def eq(e: Any): Eq[Any] = (e match
    case d: Derived[?] => d.instance
    case other => other).asInstanceOf[Eq[Any]]
  given product[A](using m: Mirror.ProductOf[A], inst: => Instances[Or, A]): Derived[Eq[A]] =
    new Derived((x, y) =>
      val (px, py) = (x.asInstanceOf[Product], y.asInstanceOf[Product])
      inst.elems().zipWithIndex.forall((e, i) => eq(e).eqv(px.productElement(i), py.productElement(i))))
  given coproduct[A](using m: Mirror.SumOf[A], inst: => Instances[Or, A]): Derived[Eq[A]] =
    new Derived((x, y) => m.ordinal(x) == m.ordinal(y) && eq(inst.elems()(m.ordinal(x))).eqv(x, y))

inline def derivedEq[A]: Eq[A] = summonInline[Derived[Eq[A]]].instance

final case class Item(id: String)
final case class Element(items: List[Item])
object Element:
  given Eq[Element] = derivedEq[Element]

@main def run(): Unit =
  val eq = summon[Eq[Element]]
  println(eq.eqv(Element(List(Item("a"), Item("b"))), Element(List(Item("a"), Item("b")))))
  println(eq.eqv(Element(List(Item("a"), Item("b"))), Element(List(Item("a"), Item("c")))))
  println(eq.eqv(Element(List(Item("a"))), Element(Nil)))
