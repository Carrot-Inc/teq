package scala

/** Evidence that `From` conforms to `To`. The compiler supplies it where a `using` parameter asks
  * for it, as the identity function.
  */
sealed trait <:<[-From, +To] extends Function1[From, To]

object <:< :
  def refl[A]: A =:= A = unsafeCast((a: A) => a)
  /** scala-library's substitutions: the evidence is the identity function at run time, so a
    * value keeps its representation and only its type changes.
    */
  extension [From, To](ev: From <:< To)
    def substituteCo[F[+_]](ff: F[From]): F[To] = unsafeCast(ff)
    def substituteContra[F[-_]](ft: F[To]): F[From] = unsafeCast(ft)
    def liftCo[F[+_]]: F[From] <:< F[To] = unsafeCast(ev)
    def liftContra[F[-_]]: F[To] <:< F[From] = unsafeCast(ev)

/** scala-library's `Predef.$conforms`: the identity where a `Function1` evidence is asked for
  * (`xss.flatten`).
  */
implicit def conforms[A]: A => A = a => a

/** Evidence that `From` and `To` are the same type. */
sealed trait =:=[From, To] extends <:<[From, To]

object =:= :
  extension [From, To](ev: From =:= To)
    def substituteBoth[F[_, _]](ftf: F[To, From]): F[From, To] = unsafeCast(ftf)
    def substituteCo[F[_]](ff: F[From]): F[To] = unsafeCast(ff)
    def substituteContra[F[_]](ft: F[To]): F[From] = unsafeCast(ft)
    def liftCo[F[_]]: F[From] =:= F[To] = unsafeCast(ev)
    def liftContra[F[_]]: F[To] =:= F[From] = unsafeCast(ev)
    def flip: To =:= From = unsafeCast(ev)

/** A type whose structural members (`T { def m: Int }`) are reached through `selectDynamic` and
  * `applyDynamic`, which the implementing class defines.
  */
trait Selectable

/** An implicit that is always there, which tells overloads apart that erase alike. */
final class DummyImplicit

object DummyImplicit:
  implicit def dummyImplicit: DummyImplicit = new DummyImplicit
