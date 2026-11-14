// A class whose higher-kinded parameter has a written bound, applied or with its parameter passed on inside its own
// definition, in one file: the parameter has the kind of its clause while the class is completed. The parameter
// passed on to a parent, the class in its own constructor and in its own parent, a sibling's first-order bound,
// another class's parameter as the argument, a class of an object and a class of a block applied before they are
// defined, a method's and an outer class's parameter as the argument, a wider inner bound. scalac 3.8.4 prints
// the line of numbers.
class K[F[_]]
class Other[F[G[_]]]
class Box[A]
class PassedOn[F[G[_]] <: AnyRef] extends Other[F] { def value = 104 }
class InCtor[F[G[_]] <: AnyRef](val next: Option[InCtor[K]]) { def value = 101 }
class InParent[F[G[_]] <: AnyRef] extends Box[InParent[K]] { def value = 102 }
class SibBound[T <: Other[H], H[G[_]] <: AnyRef] { def value = 106 }
class Later[F[G[_]] <: AnyRef] { def value = 112 }
class ParamArg[H[G[_]] <: AnyRef] extends Later[H]
object Holder {
  class Applied extends HLater[K]
  class HLater[F[G[_]] <: AnyRef] { def value = 186 }
}
def block = {
  class BK[F[_]]
  class Applied extends BLater[BK]
  class BLater[F[G[_]] <: AnyRef] { def value = 185 }
  new Applied().value
}
class MLater[F[G[_]] <: AnyRef] { def value = 601 }
def m[H[G[_]] <: AnyRef](x: MLater[H]): Int = x.value
class OLater[F[G[_]] <: AnyRef] { def value = 602 }
class Outer[H[G[_]] <: AnyRef] { class Applied extends OLater[H] }
class Base
class Sub extends Base
class LWide[F[G[X <: Base]]] { def value = 133 }
class KW[F[X <: Sub]]
class Wide extends LWide[KW]

@main def main(): Unit =
  val outer = new Outer[K]
  val values = List(
    new PassedOn[K]().value, new InCtor[K](None).value, new InParent[K]().value, new SibBound[Other[K], K]().value,
    new ParamArg[K]().value, new Holder.Applied().value, block, m(new MLater[K]), new outer.Applied().value, new Wide().value)
  println(values.mkString(" "))
