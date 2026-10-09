package fix.runtime

// Shapes of zio's runtime bodies, compiled as a library: a method-local sealed hierarchy with a
// case class and case objects, local type aliases, a var left to its default value.
object Folds:
  def fold(xs: List[Int]): String =
    sealed trait Step
    case object Both extends Step
    case object Then extends Step
    final case class Stackless(flag: Boolean) extends Step
    def loop(in: List[Int], out: List[Either[Step, String]]): List[String] =
      in match
        case 0 :: rest => loop(rest, Left(Both) :: out)
        case 1 :: rest => loop(rest, Left(Then) :: out)
        case n :: rest if n < 0 => loop(rest, Left(Stackless(n < -5)) :: out)
        case n :: rest => loop(rest, Right(n.toString) :: out)
        case Nil =>
          out.foldLeft[List[String]](Nil) {
            case (acc, Right(s)) => s :: acc
            case (acc, Left(Both)) => ("both" + acc.size) :: acc
            case (acc, Left(Then)) => "then" :: acc
            case (acc, Left(Stackless(f))) => s"stackless($f)" :: acc
          }
    loop(xs, Nil).mkString(",")

  def counters(n: Int): String =
    class Counter(start: Int, val step: Int):
      var current = start
      def next(): Int =
        current += step
        current
    val c = Counter(n, 2)
    c.next()
    s"${c.next()} ${c.step}"

  def pick[A](a: A): String =
    type A0 = A & AnyRef
    val xs: List[A0] = List(a.asInstanceOf[A0])
    xs.head.toString

  def pairs(n: Int): List[String] =
    type P[X] = (X, X)
    val ps: List[P[Int]] = List.tabulate(n)(i => (i, i * i))
    ps.map(p => s"${p._1}->${p._2}")

final class Cell:
  private[this] var value: String = _
  private[this] var count: Int = _
  def set(v: String): Unit =
    value = v
    count += 1
  def show: String = s"$value/$count"

// A trait nested in generic traits and classes, whose members name the outer type parameters
// (zio's `Metric#UnsafeAPI`, `Promise#UnsafeAPI`, `Fiber.Runtime#UnsafeAPI`).
trait Metric[-In, +Out]:
  self =>
  trait UnsafeAPI:
    def update(in: In): Unit
    def value: Out
  val unsafe: UnsafeAPI
  def contramap[In2](f: In2 => In): Metric[In2, Out] =
    new Metric[In2, Out]:
      val unsafe: UnsafeAPI = new UnsafeAPI:
        def update(in: In2): Unit = self.unsafe.update(f(in))
        def value: Out = self.unsafe.value

object Metric:
  def counter(): Metric[Long, Long] =
    new Metric[Long, Long]:
      var n = 0L
      val unsafe: UnsafeAPI = new UnsafeAPI:
        def update(in: Long): Unit = n += in
        def value: Long = n
  def lengths(): Metric[String, Long] = counter().contramap[String](_.length.toLong)
  def track(m: Metric[String, Long], s: String): Long =
    m.unsafe.update(s)
    m.unsafe.value

class Promise[E, A]:
  private var result: Option[Either[E, A]] = None
  trait UnsafeAPI:
    def completeWith(r: Either[E, A]): Boolean
    def observe(f: Either[E, A] => Unit): Unit
  val unsafe: UnsafeAPI = new UnsafeAPI:
    def completeWith(r: Either[E, A]): Boolean =
      if result.isEmpty then
        result = Some(r)
        true
      else false
    def observe(f: Either[E, A] => Unit): Unit = result.foreach(f)
  def poll: Option[Either[E, A]] = result

object Promise:
  def succeed[A](p: Promise[Nothing, A], a: A): Boolean = p.unsafe.completeWith(Right(a))
  def run[E, A](p: Promise[E, A], k: Either[E, A] => Unit): Unit = p.unsafe.observe(r => k(r))

// A type alias member over the class's parameter, seen from a subclass fixing it (zio's
// `FiberRef.Value` in `PatchFiber[Value0]`).
trait Ref[A]:
  type Value = A
  def initial: Value
  def get: List[Value] = List(initial)

final class PatchRef[V0](v: V0) extends Ref[V0]:
  def initial: Value = v
  override def get: List[Value] = List(v, initial)

object Ref:
  object unsafe:
    final class SelfRef[V0](v: V0) extends Ref[V0]:
      self =>
      def initial: Value = v
      def twice: List[Value] = List(self.initial, initial)
    def make[V](v: V): Ref[V] = new SelfRef(v)

// A union with the singleton type of an object's val (zio's `Cause.Filter.failCase`).
sealed trait Cause[+E]
object Cause:
  case object Empty extends Cause[Nothing]
  final case class Fail[+E](e: E) extends Cause[E]
  val empty: Cause[Nothing] = Empty
  final case class Filter[E](p: Cause[E] => Boolean):
    def failCase(e: E): Cause[E] =
      val c = Fail(e)
      if p(c) then c else Cause.empty

// A collection extending `IndexedSeq` and `IndexedSeqOps` over itself, whose operators give
// the collection (zio's `Chunk`).
final class Bag[+A](items: Vector[A])
    extends collection.immutable.IndexedSeq[A]
    with collection.immutable.IndexedSeqOps[A, Bag, Bag[A]]
    with collection.immutable.StrictOptimizedSeqOps[A, Bag, Bag[A]]
    with collection.IterableFactoryDefaults[A, Bag]:
  def apply(i: Int): A = items(i)
  def length: Int = items.length
  override def iterableFactory: collection.SeqFactory[Bag] = Bag
  override def className: String = "Bag"
  def plus[A1 >: A](a: A1): Bag[A1] = this :+ a
  def unique: Bag[A] = distinct

object Bag extends collection.StrictOptimizedSeqFactory[Bag]:
  def from[A](source: IterableOnce[A]): Bag[A] = new Bag(Vector.from(source))
  def empty[A]: Bag[A] = new Bag(Vector.empty)
  def newBuilder[A]: collection.mutable.Builder[A, Bag[A]] = Vector.newBuilder[A].mapResult(v => new Bag(v))

// A wildcard's capture named through a path, `head.A` of a `Slot[?]`, and a member of a
// type-aliased parameter selected on a cast receiver (zio's `FiberRefs.Patch.diff`).
final case class Slot[A](value: A)
final case class Slots(stack: ::[Slot[?]], depth: Int)
object Slots:
  sealed trait Change
  final case class Add[V0](ref: Ref[V0], value: V0) extends Change
  final case class Remove[V0](ref: Ref[V0]) extends Change
  final case class Update[V0](ref: Ref[V0], old: V0) extends Change
  def diff(pairs: Map[Ref[?], Slots]): List[Change] =
    var out = List.empty[Change]
    val it = pairs.iterator
    while it.hasNext do
      val kv = it.next()
      val ref = kv._1.asInstanceOf[Ref[Any]]
      val v = kv._2.stack.head.value
      ref.initial match
        case null => out = Add(ref, v) :: out
        case old => if old != v then out = Update(ref.asInstanceOf[Ref[ref.Value]], ref.initial) :: out
    val it1 = pairs.keysIterator
    while it1.hasNext do
      val r = it1.next()
      if r.get.isEmpty then out = Remove(r) :: out
    out.reverse

// A plain function literal whose parameter is implicit in its body (zio's
// `suspendSucceedUnsafe(implicit u => ...)`), next to a context function literal.
final class Token(val n: Int)
object Tokens:
  def need(using t: Token): Int = t.n
  def withToken[A](f: Token => A): A = f(new Token(7))
  def withGiven[A](f: Token ?=> A): A = f(using new Token(8))
  def run: String = withToken(implicit t => s"need ${need + 1}") + " " + withGiven(need.toString)
  // `Class` is `Predef.Class`, scala-library's alias of `java.lang.Class`.
  def classOfToken: Class[?] = classOf[Token]
  def sameClass(t: Token): Boolean = t.getClass == classOfToken

// A parameterised alias of a parent seen from a subclass of the same file, as the type
// constructor argument of an anonymous class's parent (zio-interop-cats' `ZioConcurrent.F`).
trait Keeper[F[_], A]:
  def held: F[A]
abstract class Effects[R, E]:
  type F[A] = Either[E, (R, A)]
  def pure[A](r: R, a: A): F[A] = Right((r, a))
abstract class Concurrents[R, E] extends Effects[R, E]:
  def hold[A](r: R, a: A): Keeper[F, A] =
    new Keeper[F, A]:
      val held: F[A] = pure(r, a)
object Concurrents:
  def ints: Concurrents[Int, String] = new Concurrents[Int, String] {}

// A class with type parameters of its own nested in a generic trait, constructed with explicit
// type arguments (scala-library's `IterableOnceOps.Maximized`).
trait Pairs[A]:
  class Pair[B](val a: A, val b: B):
    def show: String = s"$a/$b"
  def first: A
  def pair[B](b: B): Pair[B] = new Pair[B](first, b)
object Pairs:
  def ints: Pairs[Int] = new Pairs[Int] { def first = 1 }

// Bounded wildcards in invariant positions, as izumi-reflect's `LightTypeTagRef.maybeUnion`
// and `LTTRenderables.renderDb` have them.
object Wilds:
  trait Named:
    def name: String
  final case class Leaf(name: String) extends Named

  def union(r: Set[? <: Named]): String = union(r.iterator)
  def union(refs: Iterator[Named]): String = refs.map(_.name).toList.sorted.mkString("|")
  def names(refs: Iterator[? <: Named]): String = refs.map(_.name).mkString(",")
  def renderDb(db: Map[? <: Named, Set[? <: Named]]): String =
    db.toList.sortBy(_._1.name).map { case (k, v) => s"${k.name} -> ${v.toList.map(_.name).sorted.mkString(",")}" }.mkString("; ")

  final class Cell[T](val v: T)
  final case class Holder[O](items: List[Cell[? <: O]])
  def firsts(h: Any): List[String] = h match
    case s: Holder[_] => s.items.map(_.v.toString)
    case _ => Nil

// A class of a body extending a JDK class and a trait whose methods take a trailing implicit
// clause, as zio's `Ref.Atomic.unsafe` has it: the two methods of each name are overloads.
object Atomics:
  final class Token
  trait UnsafeAPI[A]:
    def get(implicit t: Token): A
    def set(a: A)(implicit t: Token): Unit
    def updateAndGet(f: A => A)(implicit t: Token): A
  def make[A](initial: A): UnsafeAPI[A] =
    new java.util.concurrent.atomic.AtomicReference[A](initial) with UnsafeAPI[A]:
      def get(implicit t: Token): A = this.asInstanceOf[java.util.concurrent.atomic.AtomicReference[A]].get()
      def set(a: A)(implicit t: Token): Unit = this.asInstanceOf[java.util.concurrent.atomic.AtomicReference[A]].set(a)
      def updateAndGet(f: A => A)(implicit t: Token): A =
        this.asInstanceOf[java.util.concurrent.atomic.AtomicReference[A]].updateAndGet(v => f(v))

// A case class with default arguments built from outside the jar, as izumi-reflect's
// `NameReference(ref)`.
object Refs:
  sealed trait Bounds
  object Bounds:
    case object Empty extends Bounds
  final case class Named(ref: String, bounds: Bounds = Bounds.Empty, prefix: Option[Named] = None)
  object Named:
    private[Named] def apply(n: Int): Named = Named(n.toString)
  def single(key: String): Map[String, Int] = new Predef.Map.Map1(key, 1)

// An inspector that makes the next one as an anonymous class refining a `val` over the
// enclosing class's path, as izumi-reflect's `Inspector.next` does.
object Inspectors:
  final case class Lam(input: List[String], output: String)
  abstract class Insp(val depth: Int, val context: List[String]):
    val q: AnyRef
    def next(extra: Option[String] = None): Insp { val q: Insp.this.q.type } =
      new Insp(depth + 1, extra.fold(context)(context :+ _)) { val q: Insp.this.q.type = Insp.this.q }
    def nextLam(n: Int): Insp { val q: Insp.this.q.type } =
      next(Some((0 until n).map(i => s"p$i").mkString(",")))
    def inspect(t: Any): String = t match
      case n: Int =>
        val inspector = nextLam(n)
        val resType = inspector.inspect(s"body$n")
        val paramNames = inspector.context.last.split(",").toList
        Lam(paramNames, resType).toString
      case s: String => s + "@" + depth
      case _ => "?"
  def run(): String = new Insp(0, Nil) { val q: AnyRef = "q" }.inspect(2)

// A trait whose val implements an ancestor's def, mixed into a program's object: the val is
// set when the object is made, as zio's `ZIOAppDefault.bootstrap` is.
object Apps:
  trait App:
    def bootstrap: String
    def run: String = "run " + bootstrap
  trait DefaultApp extends App:
    val bootstrap: String = "empty"

// An object in a trait's companion that extends the trait and makes the trait's nested trait,
// as zio's `Clock.ClockLive.unsafe` is: the outer instance is the object itself.
object Clocks:
  trait Clock:
    trait UnsafeAPI:
      def now(): Long
    def unsafe: UnsafeAPI
  object Clock:
    object Live extends Clock:
      override val unsafe: UnsafeAPI = new UnsafeAPI:
        def now(): Long = 42L
