// expect: covariant type A occurs in contravariant position in type A of parameter a
// expect: 1 error found
import scala.annotation.unchecked.uncheckedVariance

trait Box[+A]:
  def keep(a: A @uncheckedVariance): Box[A] = this
  def drop(a: A): Box[A] = this

@main def m(): Unit = ()
