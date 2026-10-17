// A lambda whose result is the contextual type: its body's type with the parameters avoided
// (`TypeOps.avoid`) has to conform to it (Namer.scala 1912–1919), and an instance avoids a parameter
// deeper than its variable (`ConstraintHandling.fixLevels`): `x => Item(x.a)` against `S => Item[A]`
// cannot be made hygienic (`Item[?]` is no `Item[A]`), and the cons of `map`'s `Item[?]` is no
// `List[Item[Any]]`, as scalac refuses both. A union avoids as `ApproximatingTypeMap.derivedOrType`
// composes it, the ranges of its parts joined before the invariant argument takes the result:
// `Item[x.A | String]` is `Item[? >: String]`.
final case class Item[T](value: T)
trait S { type A; val a: A }
def pair[A](f: S => Item[A], seed: A): Item[A] = Item(seed)
def first(s: S) = pair(x => Item(x.a), "later").value
def cons(s: S) =
  val xs: List[Item[Any]] = Item[Any](1) :: List(s).map(x => Item(x.a))
  xs
def joined(s: S) = pair(x => Item((x.a: x.A | String)), "later").value
// expect: 11:24: error: return type Item[Any] of lambda cannot be made hygienic
// expect: it is not a supertype of the hygienic type Item[?]
// expect: 13:29: error: type mismatch: found List[Item[?]], required List[Item[Any]]
// expect: 15:25: error: return type Item[Any] of lambda cannot be made hygienic
// expect: it is not a supertype of the hygienic type Item[? >: String]
