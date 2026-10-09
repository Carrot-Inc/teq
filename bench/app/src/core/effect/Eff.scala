package meridian.core.effect

/** An effect with an environment, an error channel and a result, run by [[Runtime]] on a
  * trampoline so that a long chain of flatMaps costs no stack. */
sealed trait Eff[-R, +E, +A]:
  def flatMap[R1 <: R, E1 >: E, B](f: A => Eff[R1, E1, B]): Eff[R1, E1, B] = Eff.Bind(this, f)
  def map[B](f: A => B): Eff[R, E, B] = flatMap(a => Eff.Done(f(a)))
  def as[B](b: => B): Eff[R, E, B] = map(_ => b)
  def unit: Eff[R, E, Unit] = map(_ => ())
  def zip[R1 <: R, E1 >: E, B](that: Eff[R1, E1, B]): Eff[R1, E1, (A, B)] = flatMap(a => that.map(b => (a, b)))
  def zipRight[R1 <: R, E1 >: E, B](that: Eff[R1, E1, B]): Eff[R1, E1, B] = flatMap(_ => that)
  def *>[R1 <: R, E1 >: E, B](that: Eff[R1, E1, B]): Eff[R1, E1, B] = zipRight(that)
  def <*[R1 <: R, E1 >: E, B](that: Eff[R1, E1, B]): Eff[R1, E1, A] = flatMap(a => that.as(a))
  def tap[R1 <: R, E1 >: E](f: A => Eff[R1, E1, Any]): Eff[R1, E1, A] = flatMap(a => f(a).as(a))
  def foldEff[R1 <: R, E2, B](onError: E => Eff[R1, E2, B], onSuccess: A => Eff[R1, E2, B]): Eff[R1, E2, B] =
    Eff.Fold(this, onError, onSuccess)
  def catchAll[R1 <: R, E2, A1 >: A](h: E => Eff[R1, E2, A1]): Eff[R1, E2, A1] = foldEff(h, a => Eff.Done(a))
  def mapError[E2](f: E => E2): Eff[R, E2, A] = foldEff(e => Eff.Fail(f(e)), a => Eff.Done(a))
  def orElse[R1 <: R, E2, A1 >: A](that: => Eff[R1, E2, A1]): Eff[R1, E2, A1] = catchAll(_ => that)
  def either: Eff[R, Nothing, Either[E, A]] = foldEff(e => Eff.Done(Left(e)), a => Eff.Done(Right(a)))
  def option: Eff[R, Nothing, Option[A]] = foldEff(_ => Eff.Done(None), a => Eff.Done(Some(a)))
  def ignore: Eff[R, Nothing, Unit] = foldEff(_ => Eff.unit, _ => Eff.unit)
  def when(condition: Boolean): Eff[R, E, Unit] = if condition then unit else Eff.unit
  def unless(condition: Boolean): Eff[R, E, Unit] = when(!condition)
  def provide(env: Env[R]): Eff[Any, E, A] = Eff.Provide(this, env)
  def provideLayer[R0](layer: Layer[R0, R]): Eff[R0, E, A] = layer.build.flatMap(env => provide(env))
  def provideSome[R0, R1](env: Env[R1])(using ev: (R0 & R1) <:< R): Eff[R0, E, A] =
    Eff.environment[R0].flatMap(outer => Eff.Provide(this, (outer ++ env).asInstanceOf[Env[R]]))
  def someOrFail[B, E1 >: E](error: => E1)(using ev: A <:< Option[B]): Eff[R, E1, B] =
    flatMap(a => ev(a) match
      case Some(b) => Eff.Done(b)
      case None => Eff.Fail(error))
  def absolve[E1 >: E, B](using ev: A <:< Either[E1, B]): Eff[R, E1, B] =
    flatMap(a => ev(a) match
      case Right(b) => Eff.Done(b)
      case Left(e) => Eff.Fail(e))
  def retry(times: Int): Eff[R, E, A] = if times <= 0 then this else catchAll(_ => retry(times - 1))
  def timed: Eff[R, E, (Long, A)] = Eff.clock.flatMap(start => map(a => (Eff.tick() - start, a)))

object Eff:
  final case class Done[A](value: A) extends Eff[Any, Nothing, A]
  final case class Fail[E](error: E) extends Eff[Any, E, Nothing]
  final case class Suspend[R, E, A](thunk: () => Eff[R, E, A]) extends Eff[R, E, A]
  final case class Bind[R, E, A, B](source: Eff[R, E, A], next: A => Eff[R, E, B]) extends Eff[R, E, B]
  final case class Fold[R, E, E2, A, B](source: Eff[R, E, A], onError: E => Eff[R, E2, B], onSuccess: A => Eff[R, E2, B]) extends Eff[R, E2, B]
  final case class Access[R](read: Env[R] => Any) extends Eff[R, Nothing, Any]
  final case class Provide[R, E, A](source: Eff[R, E, A], env: Env[R]) extends Eff[Any, E, A]

  private var ticks: Long = 0L
  private[effect] def tick(): Long =
    ticks += 1
    ticks

  val unit: Eff[Any, Nothing, Unit] = Done(())
  def succeed[A](a: => A): Eff[Any, Nothing, A] = Suspend(() => Done(a))
  def pure[A](a: A): Eff[Any, Nothing, A] = Done(a)
  def fail[E](e: => E): Eff[Any, E, Nothing] = Suspend(() => Fail(e))
  def suspend[R, E, A](eff: => Eff[R, E, A]): Eff[R, E, A] = Suspend(() => eff)
  def attempt[A](a: => A): Eff[Any, Throwable, A] =
    Suspend(() =>
      try Done(a)
      catch case e: Throwable => Fail(e))
  def fromEither[E, A](either: => Either[E, A]): Eff[Any, E, A] =
    Suspend(() => either match
      case Right(a) => Done(a)
      case Left(e) => Fail(e))
  def fromOption[A](option: => Option[A]): Eff[Any, Unit, A] =
    Suspend(() => option match
      case Some(a) => Done(a)
      case None => Fail(()))
  def when[R, E](condition: Boolean)(eff: => Eff[R, E, Any]): Eff[R, E, Unit] = if condition then eff.unit else unit
  def unless[R, E](condition: Boolean)(eff: => Eff[R, E, Any]): Eff[R, E, Unit] = when(!condition)(eff)
  def cond[E, A](test: Boolean, result: => A, error: => E): Eff[Any, E, A] = if test then succeed(result) else fail(error)
  def clock: Eff[Any, Nothing, Long] = succeed(tick())
  def service[R](using tag: Tag[R]): Eff[R, Nothing, R] = Access[R](env => env.get[R](using tag)).asInstanceOf[Eff[R, Nothing, R]]
  def serviceWith[R](using tag: Tag[R]): ServiceWith[R] = ServiceWith[R](tag)
  def environment[R]: Eff[R, Nothing, Env[R]] = Access[R](identity).asInstanceOf[Eff[R, Nothing, Env[R]]]

  def foreach[R, E, A, B](as: List[A])(f: A => Eff[R, E, B]): Eff[R, E, List[B]] =
    as.foldRight[Eff[R, E, List[B]]](Done(Nil))((a, acc) => f(a).flatMap(b => acc.map(bs => b :: bs)))
  def foreachDiscard[R, E, A](as: Iterable[A])(f: A => Eff[R, E, Any]): Eff[R, E, Unit] =
    as.foldLeft[Eff[R, E, Unit]](unit)((acc, a) => acc.flatMap(_ => f(a).unit))
  def foreachOption[R, E, A, B](a: Option[A])(f: A => Eff[R, E, B]): Eff[R, E, Option[B]] = a match
    case Some(v) => f(v).map(Some(_))
    case None => Done(None)
  def collectAll[R, E, A](effs: List[Eff[R, E, A]]): Eff[R, E, List[A]] = foreach(effs)(identity)
  def collectAllDiscard[R, E](effs: List[Eff[R, E, Any]]): Eff[R, E, Unit] = foreachDiscard(effs)(identity)
  def loop[R, E, S](start: S)(continue: S => Boolean, step: S => Eff[R, E, S]): Eff[R, E, S] =
    if continue(start) then step(start).flatMap(s => loop(s)(continue, step)) else Done(start)

  final class ServiceWith[R](tag: Tag[R]):
    def apply[A](f: R => A): Eff[R, Nothing, A] = service[R](using tag).map(f)
    def eff[R1, E, A](f: R => Eff[R1, E, A]): Eff[R & R1, E, A] = service[R](using tag).flatMap(f)

type Task[+A] = Eff[Any, Throwable, A]
type UIO[+A] = Eff[Any, Nothing, A]
type IO[+E, +A] = Eff[Any, E, A]
type RIO[-R, +A] = Eff[R, Throwable, A]
type URIO[-R, +A] = Eff[R, Nothing, A]

/** A name for a service type, filled in by the environment builders. */
final class Tag[A](val name: String):
  override def toString = s"Tag($name)"
object Tag:
  def apply[A](name: String): Tag[A] = new Tag[A](name)
  inline given derived[A]: Tag[A] = ${ TagMacros.derivedImpl[A] }

/** An environment of services keyed by their tags; `R` is the intersection of the service types. */
final class Env[+R] private (private[effect] val services: Map[String, Any]):
  def ++[R1](that: Env[R1]): Env[R & R1] = new Env(services ++ that.services)
  def add[S](service: S)(using tag: Tag[S]): Env[R & S] = new Env(services.updated(tag.name, service))
  def get[S](using tag: Tag[S]): S =
    services.getOrElse(tag.name, throw new NoSuchElementException(s"no service ${tag.name} in $this")).asInstanceOf[S]
  def has[S](using tag: Tag[S]): Boolean = services.contains(tag.name)
  override def toString = services.keys.toList.sorted.mkString("Env(", ", ", ")")
object Env:
  val empty: Env[Any] = new Env(Map.empty)
  def apply[S](service: S)(using tag: Tag[S]): Env[S] = empty.add(service)
  def apply[S1, S2](s1: S1, s2: S2)(using t1: Tag[S1], t2: Tag[S2]): Env[S1 & S2] = empty.add(s1).add(s2)

/** A recipe for an environment, composed with `++` and given to `provideLayer`. */
final class Layer[-RIn, +ROut](val build: Eff[RIn, Nothing, Env[ROut]]):
  def ++[RIn1 <: RIn, ROut1](that: Layer[RIn1, ROut1]): Layer[RIn1, ROut & ROut1] =
    Layer(build.zip(that.build).map((a, b) => a ++ b))
  def >>>[ROut2](that: Layer[ROut, ROut2]): Layer[RIn, ROut2] = Layer(build.flatMap(env => that.build.provide(env)))
object Layer:
  def succeed[S](service: => S)(using tag: Tag[S]): Layer[Any, S] = Layer(Eff.succeed(Env(service)))
  def fromEff[RIn, S](eff: Eff[RIn, Nothing, S])(using tag: Tag[S]): Layer[RIn, S] = Layer(eff.map(s => Env(s)))
  def fromFunction[RIn, S](f: Env[RIn] => S)(using tag: Tag[S]): Layer[RIn, S] = Layer(Eff.environment[RIn].map(env => Env(f(env))))
  def environment[R]: Layer[R, R] = Layer(Eff.environment[R])

/** A mutable cell in the effect world. */
final class Ref[A] private (private var value: A):
  def get: UIO[A] = Eff.succeed(value)
  def set(a: A): UIO[Unit] = Eff.succeed { value = a }
  def update(f: A => A): UIO[Unit] = Eff.succeed { value = f(value) }
  def updateAndGet(f: A => A): UIO[A] = Eff.succeed { value = f(value); value }
  def getAndUpdate(f: A => A): UIO[A] = Eff.succeed { val old = value; value = f(value); old }
  def modify[B](f: A => (B, A)): UIO[B] = Eff.succeed { val (b, a) = f(value); value = a; b }
  def unsafeGet: A = value
object Ref:
  def make[A](a: A): UIO[Ref[A]] = Eff.succeed(new Ref(a))
  def unsafeMake[A](a: A): Ref[A] = new Ref(a)

/** A queue with a bounded history, the shape of a message channel between services. */
final class Queue[A] private (private var items: List[A]):
  def offer(a: A): UIO[Unit] = Eff.succeed { items = items :+ a }
  def take: IO[Unit, A] = Eff.suspend(items match
    case head :: tail => Eff.succeed { items = tail; head }
    case Nil => Eff.fail(()))
  def size: UIO[Int] = Eff.succeed(items.length)
  def drain: UIO[List[A]] = Eff.succeed { val all = items; items = Nil; all }
object Queue:
  def unbounded[A]: UIO[Queue[A]] = Eff.succeed(new Queue(Nil))
