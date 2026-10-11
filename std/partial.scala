package scala

// A partial function is a JS function that carries `isDefinedAt` and `applyOrElse`, so it can be
// passed wherever an `A => B` is expected: a `{ case ... }` literal is one `newPartialFunction`
// makes, an instance of a class extending the trait (a program's, a library's, the combinations
// below) one its constructor makes, as of any class with a function type among its parents.
trait PartialFunction[-A, +B] extends Function1[A, B]:
  @js("$0($1)")
  @jvm("$0 $1:L invokeinterface scala/Function1.apply(Ljava/lang/Object;)Ljava/lang/Object;")
  def apply(x: A): B
  def isDefinedAt(x: A): Boolean
  def applyOrElse[A1 <: A, B1 >: B](x: A1, default: A1 => B1): B1 =
    if isDefinedAt(x) then apply(x) else default(x)

  def lift: A => Option[B] = x =>
    val v = applyOrElse(x, partialMiss[A, B])
    if isPartialMiss(v) then None else Some(v)

  def orElse[A1 <: A, B1 >: B](that: PartialFunction[A1, B1]): PartialFunction[A1, B1] = new PartialFunction.OrElse(this, that)

  // A partial function as `k` narrows the domain to the values whose image it accepts.
  def andThen[C](k: B => C): PartialFunction[A, C] = k match
    case pf: PartialFunction[?, ?] => andThen(unsafeCast[PartialFunction[?, ?], PartialFunction[B, C]](pf))
    case _ => new PartialFunction.AndThen(this, k)
  def andThen[C](k: PartialFunction[B, C]): PartialFunction[A, C] = new PartialFunction.Combined(this, k)

  def compose[R](k: PartialFunction[R, A]): PartialFunction[R, B] = new PartialFunction.Combined(k, this)

  def runWith[U](action: B => U): A => Boolean = x =>
    val v = applyOrElse(x, partialMiss[A, B])
    if isPartialMiss(v) then false
    else
      action(v)
      true

@js("$pf($0, $1)")
def newPartialFunction[A, B](applyOrElse: (A, A => B) => B, isDefinedAt: A => Boolean): PartialFunction[A, B] = new scala.runtime.PartialFunctionImpl(applyOrElse, isDefinedAt)

// The default that `applyOrElse` gets from callers that have to tell "no case matched" from a
// result without running the patterns twice.
@js("$pfMiss")
def partialMiss[A, B]: A => B = unsafeCast(scala.runtime.missFunction)

@js("($0 === $pfMissed)")
def isPartialMiss(x: Any): Boolean = x eq scala.runtime.Missed

object PartialFunction:
  // scala-library's combinations, whose `apply` calls the parts' `apply` and whose
  // `applyOrElse` runs each part's once.
  private final class OrElse[-A, +B](f1: PartialFunction[A, B], f2: PartialFunction[A, B]) extends PartialFunction[A, B]:
    def isDefinedAt(x: A): Boolean = f1.isDefinedAt(x) || f2.isDefinedAt(x)
    def apply(x: A): B = f1.applyOrElse(x, f2)
    override def applyOrElse[A1 <: A, B1 >: B](x: A1, default: A1 => B1): B1 =
      val z = f1.applyOrElse(x, partialMiss[A, B])
      if !isPartialMiss(z) then z else f2.applyOrElse(x, default)
    override def orElse[A1 <: A, B1 >: B](that: PartialFunction[A1, B1]): PartialFunction[A1, B1] = new OrElse(f1, f2.orElse(that))
    override def andThen[C](k: B => C): PartialFunction[A, C] = new OrElse(f1.andThen(k), f2.andThen(k))
  private final class AndThen[-A, B, +C](pf: PartialFunction[A, B], k: B => C) extends PartialFunction[A, C]:
    def isDefinedAt(x: A): Boolean = pf.isDefinedAt(x)
    def apply(x: A): C = k(pf(x))
    override def applyOrElse[A1 <: A, C1 >: C](x: A1, default: A1 => C1): C1 =
      val z = pf.applyOrElse(x, partialMiss[A, B])
      if !isPartialMiss(z) then k(z) else default(x)
  private final class Combined[-A, B, +C](pf: PartialFunction[A, B], k: PartialFunction[B, C]) extends PartialFunction[A, C]:
    def isDefinedAt(x: A): Boolean =
      val b = pf.applyOrElse(x, partialMiss[A, B])
      !isPartialMiss(b) && k.isDefinedAt(b)
    def apply(x: A): C = k(pf(x))
    override def applyOrElse[A1 <: A, C1 >: C](x: A1, default: A1 => C1): C1 =
      val v = pf.applyOrElse(x, partialMiss[A, B])
      if !isPartialMiss(v) then k.applyOrElse(v, (_: B) => default(x)) else default(x)
  def fromFunction[A, B](f: A => B): PartialFunction[A, B] = newPartialFunction((x, default) => f(x), x => true)
  def empty[A, B]: PartialFunction[A, B] = newPartialFunction((x, default) => default(x), x => false)
  def cond[A](x: A)(pf: PartialFunction[A, Boolean]): Boolean = pf.applyOrElse(x, _ => false)
  def condOpt[A, B](x: A)(pf: PartialFunction[A, B]): Option[B] = pf.lift(x)

def unliftFunction[A, B](f: A => Option[B]): PartialFunction[A, B] =
  newPartialFunction(
    (x, default) =>
      f(x) match
        case Some(v) => v
        case None => default(x),
    x => f(x).isDefined
  )

extension [A, B](f: A => Option[B])
  def unlift: PartialFunction[A, B] = unliftFunction(f)
