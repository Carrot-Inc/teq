// `summonInline` of a type class through a higher-kinded parameter, a member alias over it and
// a type lambda argument, inside inline methods of a trait that an object instantiates.
package inlinehkaliassummon

import scala.compiletime.*

trait TC[T]
final class CBN[+A](a: () => A):
  def value: A = a()

object P:
  def apply[F[_], T, Q](name: String, cbn: CBN[F[Q]], anns: IArray[Any]): String = name + anns.length

trait Common[TypeClass[_]]:
  type Typeclass[T] = TypeClass[T]
  type Mono = TypeClass[String]
  inline def direct[p]: String = summonInline[TypeClass[p]].toString.take(2)
  inline def mono: String = summonInline[Mono].toString.take(2)
  inline def viaAlias[p]: String = summonInline[Typeclass[p]].toString.take(2)
  inline def viaParam[F[_], p](inline x: Int): String = summonInline[F[p]].toString.take(2) + x
  inline def hk[p]: String = viaParam[Typeclass, p](1)
  inline def mk[A, p](name: String): String =
    P.apply[Typeclass, A, p](name, new CBN(() => summonInline[Typeclass[p]]), IArray.from(List[Any](1, 2)))

object D extends Common[TC]
given TC[String] = new TC[String] { override def toString = "tc-string" }

@main def main(): Unit =
  println(D.direct[String])
  println(D.mono)
  println(D.viaAlias[String])
  println(D.hk[String])
  println(D.mk[Int, String]("x"))
