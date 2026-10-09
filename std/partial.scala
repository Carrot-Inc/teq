package scala

// A partial function is a JS function that carries `isDefinedAt` and `applyOrElse`, so it can be
// passed wherever an `A => B` is expected. Every instance comes from `newPartialFunction`, which
// is also what a `{ case ... }` literal compiles to.
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

  def orElse[A1 <: A, B1 >: B](that: PartialFunction[A1, B1]): PartialFunction[A1, B1] =
    newPartialFunction(
      (x, default) =>
        val v = applyOrElse(x, partialMiss[A1, B1])
        if isPartialMiss(v) then that.applyOrElse(x, default) else v,
      x => isDefinedAt(x) || that.isDefinedAt(x)
    )

  def andThen[C](k: B => C): PartialFunction[A, C] =
    newPartialFunction(
      (x, default) =>
        val v = applyOrElse(x, partialMiss[A, B])
        if isPartialMiss(v) then default(x) else k(v),
      x => isDefinedAt(x)
    )

  // A partial function as `k` narrows the domain to the values whose image it accepts.
  def andThen[C](k: PartialFunction[B, C]): PartialFunction[A, C] = combinePartial(this, k)

  // A plain function as `k` goes to the `compose` of functions, whose result is a function.
  def compose[R](k: PartialFunction[R, A]): PartialFunction[R, B] = combinePartial(k, this)

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

def combinePartial[A, B, C](first: PartialFunction[A, B], second: PartialFunction[B, C]): PartialFunction[A, C] =
  newPartialFunction(
    (x, default) =>
      val v = first.applyOrElse(x, partialMiss[A, B])
      if isPartialMiss(v) then default(x) else second.applyOrElse(v, _ => default(x)),
    x =>
      val v = first.applyOrElse(x, partialMiss[A, B])
      !isPartialMiss(v) && second.isDefinedAt(v)
  )

object PartialFunction:
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
