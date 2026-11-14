// A class whose higher-kinded parameter has a written bound, or a clause whose own parameter has one, first applied
// in the file typed before the class's: the parameter has the kind of its clause from the start, so each
// argument below is of its kind, and the bound is read once it is resolved. Each class of the other file is
// applied here once: an upper bound, a lower and an upper, a trait, a clause of two, a bound that names a class of
// the program, the class itself, a variance, a clause's inner bound with and without a bound, a wider inner
// bound, a clause nested twice, a context bound, a type lambda and an alias as the argument, a bound and an inner
// bound that name a sibling parameter, an inner lower and upper bound, an enum, an outer class's parameter and
// another class's parameter as the argument (in a parent, a constructor and a sibling's bound), an alias bound of
// the parameter itself, two traits each in the other's bound; and a lower bound alone, a first-order bound and a
// signature. scalac 3.8.4 prints the line of numbers.
package earlykind

class K[F[_]]
class KT[F[_], A]
class KB[F[_]] extends Base
class KS[F[_]] extends LSelf[KS]
class KV[+F[_]]
class KI[F[X <: Base]]
class KW[F[X <: Sub]]
class KN[F[G[_]]]
class KC[F[_]]
given TC[KC] = new TC[KC] {}
type KA[F[_]] = K[F]
class KStr[F[X <: String]]
class Lo extends Hi
class Hi
class KLH[F[X >: Lo <: Hi]]

class Upper extends LUpper[K]
class Both extends LBoth[K]
class OfTrait extends LTrait[K]
class TwoParams extends LTwo[KT]
class NamesClass extends LBase[KB]
class Variance extends LVar[KV]
class Inner extends LInner[KI]
class InnerBound extends LInnerB[KI]
class Wider extends LWider[KW]
class Nested extends LNested[KN]
class Ctx extends LCtx[KC]
class Lam extends LLam[[G[_]] =>> K[G]]
class Ali extends LAli[KA]
class Sib extends LSib[AnyRef, K]
class SibIn extends LSibIn[String, KStr]
class LoHi extends LLoHi[KLH]
class EnumUse(val l: LEnum[K]) { def value = 81 }
class Outer[H[G[_]] <: AnyRef] { class In extends LOut[H] }
class ParamArg[H[G[_]] <: AnyRef] extends LParam[H]
class NoBound[H[G[_]] <: AnyRef] extends LNoBound[H]
class AliasBound extends LAliasB[K]
class Each extends One[KO]
class InCtor[H[G[_]] <: AnyRef](val x: LCtor[H]) { def value = x.value }
class InBound[H[G[_]] <: AnyRef, T <: LSibB[H]](val t: T) { def value = t.value }
class LowerOnly extends LLower[K]
class FirstOrder extends LFirst[List]
object InSig { def f(l: LSig[K]): Int = l.value }

@main def main(): Unit =
  val outer = new Outer[K]
  val values = List(
    new Upper().value, new Both().value, new OfTrait().value, new TwoParams().value, new NamesClass().value,
    new KS[List]().value, new Variance().value, new Inner().value, new InnerBound().value, new Wider().value,
    new Nested().value, new Ctx().value, new Lam().value, new Ali().value, new Sib().value, new SibIn().value,
    new LoHi().value, new EnumUse(LEnum.One[K]()).value, new outer.In().value, new ParamArg[K]().value,
    new NoBound[K]().value, new AliasBound().value, new Each().value, new InCtor[K](new LCtor[K]).value,
    new InBound[K, LSibB[K]](new LSibB[K]).value, new LowerOnly().value, new FirstOrder().value, InSig.f(new LSig[K]))
  println(values.mkString(" "))
