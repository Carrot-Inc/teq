package lib.kernel

trait Eq[A]:
  def eqv(x: A, y: A): Boolean

object Eq:
  def instance[A](f: (A, A) => Boolean): Eq[A] = new Eq[A]:
    def eqv(x: A, y: A): Boolean = f(x, y)
