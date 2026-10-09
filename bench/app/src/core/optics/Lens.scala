package meridian.core.optics

/** One optic for every shape the applications use: a lens has exactly one target, an optional
  * at most one, a traversal any number. `getAll` and `modify` carry all three. */
final class Optic[S, A](val getAll: S => List[A], val modifyWith: (A => A) => S => S):
  def get(s: S): A = getAll(s).head
  def getOption(s: S): Option[A] = getAll(s).headOption
  def modify(f: A => A): S => S = modifyWith(f)
  def replace(a: A): S => S = modifyWith(_ => a)
  def andThen[B](that: Optic[A, B]): Optic[S, B] =
    Optic(s => getAll(s).flatMap(that.getAll), f => modifyWith(that.modifyWith(f)))
  def filter(p: A => Boolean): Optic[S, A] =
    Optic(s => getAll(s).filter(p), f => modifyWith(a => if p(a) then f(a) else a))
  def exists(p: A => Boolean)(s: S): Boolean = getAll(s).exists(p)

type Lens[S, A] = Optic[S, A]
type Optional[S, A] = Optic[S, A]
type Traversal[S, A] = Optic[S, A]
type Setter[S, A] = Optic[S, A]

object Lens:
  def apply[S, A](get: S => A)(replace: A => S => S): Lens[S, A] =
    Optic(s => List(get(s)), f => s => replace(f(get(s)))(s))
  def id[S]: Lens[S, S] = Optic(s => List(s), f => f)
  transparent inline def gen[S]: LensGen[S] = LensGen[S]()
  final class LensGen[S]:
    inline def apply[A](inline path: S => A): Lens[S, A] = ${ LensMacro.gen[S, A]('path) }

object Optional:
  def apply[S, A](getOption: S => Option[A])(replace: A => S => S): Optional[S, A] =
    Optic(s => getOption(s).toList, f => s => getOption(s).map(a => replace(f(a))(s)).getOrElse(s))

object Iso:
  def id[S]: Lens[S, S] = Lens.id[S]

extension [S, K, V](optic: Optic[S, Map[K, V]])
  def at(key: K): Lens[S, Option[V]] =
    optic.andThen(Lens[Map[K, V], Option[V]](_.get(key))(v => m => v.map(x => m.updated(key, x)).getOrElse(m.removed(key))))
  def index(key: K): Optional[S, V] =
    optic.andThen(Optional[Map[K, V], V](_.get(key))(v => m => if m.contains(key) then m.updated(key, v) else m))
  def eachValue: Traversal[S, V] =
    optic.andThen(Optic[Map[K, V], V](_.values.toList, f => m => m.map((k, v) => (k, f(v)))))

extension [S, A](optic: Optic[S, List[A]])
  def each: Traversal[S, A] = optic.andThen(Optic[List[A], A](identity, f => _.map(f)))
  def headOption: Optional[S, A] = optic.andThen(Optional[List[A], A](_.headOption)(a => xs => if xs.isEmpty then xs else a :: xs.tail))

extension [S, A](optic: Optic[S, Option[A]])
  def some: Optional[S, A] = optic.andThen(Optional[Option[A], A](identity)(a => _ => Some(a)))
  def withDefault(default: A): Lens[S, A] = optic.andThen(Lens[Option[A], A](_.getOrElse(default))(a => _ => Some(a)))
