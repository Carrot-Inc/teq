// Scala.js's `Option`-like operations on `js.UndefOr`, which Scala 3 reaches through
// `js.internal.UnitOps.unitOrOps` for a member selected on an `A | Unit`.
package scala.scalajs.js:

  final class UndefOrOps[A](self: UndefOr[A]):
    def isEmpty: Boolean = isUndefined(self)
    def isDefined: Boolean = !isEmpty
    def nonEmpty: Boolean = !isEmpty
    def get: A = if isEmpty then throw new NoSuchElementException("undefined.get") else self.asInstanceOf[A]
    def getOrElse[B >: A](default: => B): B = if isEmpty then default else get
    def orNull[A1 >: A](implicit ev: Null <:< A1): A1 = if isEmpty then ev(null) else get
    def map[B](f: A => B): UndefOr[B] = if isEmpty then undefined else f(get)
    def fold[B](ifEmpty: => B)(f: A => B): B = if isEmpty then ifEmpty else f(get)
    def flatMap[B](f: A => UndefOr[B]): UndefOr[B] = if isEmpty then undefined else f(get)
    def filter(p: A => Boolean): UndefOr[A] = if isEmpty || p(get) then self else undefined
    def filterNot(p: A => Boolean): UndefOr[A] = if isEmpty || !p(get) then self else undefined
    def exists(p: A => Boolean): Boolean = !isEmpty && p(get)
    def forall(p: A => Boolean): Boolean = isEmpty || p(get)
    def foreach[U](f: A => U): Unit = if !isEmpty then f(get)
    def contains[A1 >: A](elem: A1): Boolean = !isEmpty && elem == get
    def orElse[B >: A](alternative: => UndefOr[B]): UndefOr[B] = if isEmpty then alternative else self
    def toOption: Option[A] = if isEmpty then None else Some(get)
    def toList: List[A] = if isEmpty then Nil else List(get)

package scala.scalajs.js.internal:

  object UnitOps:
    implicit def unitOrOps[A](x: A | Unit): scala.scalajs.js.UndefOrOps[A] = new scala.scalajs.js.UndefOrOps(x)
