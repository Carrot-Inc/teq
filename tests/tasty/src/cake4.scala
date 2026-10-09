package fix.cake

// More of chimney's shapes: an alias of an object nested in a trait read through the self
// type of another (`Mod.this.ExistentialType.Bounded`), the constructor of a class nested in a
// trait extended from a trait whose self type derives from it, the singleton of an inner
// class's member through a pattern's value, a member selected on an `asInstanceOf`, a
// projection that its lambdas' parameters are typed with, a class mixing in a trait nested in
// an object of its enclosing trait, a case class a base trait nests matched from a self-typed
// trait, a class's private members read from its companion, both nested in a trait, a case
// class's companion `unapply` over an abstract type, and a method parameter of a local class
// shadowing the lambda parameter its type names.
trait FoTypes { self: FoExs =>
  type Type[A]
  def tpe[A](using t: Type[A]): String
}

trait FoExs { self: FoTypes =>
  object Existential {
    trait Bounded[L, U >: L, F[_ >: L <: U]] {
      type Underlying >: L <: U
      implicit val Underlying: Type[Underlying]
      val value: F[Underlying]
      def mapK[G[_ >: L <: U]](f: Type[Underlying] => F[Underlying] => G[Underlying]): Bounded[L, U, G] =
        Existential.make[L, U, G, Underlying](f(Underlying)(value))(Underlying)
    }
    def make[L, U >: L, G[_ >: L <: U], A >: L <: U](v: G[A])(t: Type[A]): Bounded[L, U, G] =
      new Bounded[L, U, G] { type Underlying = A; val Underlying = t; val value = v }
    def apply[F[_], A](v: F[A])(using t: Type[A]): Bounded[Nothing, Any, F] = make[Nothing, Any, F, A](v)(t)
  }
  private class Impl[L, U >: L, F[_ >: L <: U], A >: L <: U](val Underlying: Type[A], val value: F[A]) extends Existential.Bounded[L, U, F] {
    type Underlying = A
  }
  def viaImpl[A](a: A)(using t: Type[A]): String = {
    val b: Existential.Bounded[Nothing, Any, [X] =>> X] = new Impl[Nothing, Any, [X] =>> X, A](t, a)
    b.mapK[Option](_ => v => Some(v)).value.toString
  }
  type ExistentialType = ExistentialType.Bounded[Nothing, Any]
  object ExistentialType {
    type Bounded[L, U >: L] = Existential.Bounded[L, U, Type]
    def apply[A](using t: Type[A]): Bounded[Nothing, Any] = Existential.apply[Type, A](t)
  }
}

trait FoDefs extends FoTypes, FoExs, FoHier, FoPaths
trait FoDeriv extends FoDefs, FoMod, FoOuters, FoMod2, FoHierUse

trait FoHier { self: FoTypes =>
  final case class En[A](elements: List[String])
  protected val SH: SHModule
  protected trait SHModule { self: SH.type =>
    def parse[A](implicit t: Type[A]): Option[En[A]]
    final def unapply[A](t: Type[A]): Option[En[A]] = parse[A](using t)
  }
  final case class Ctor[A](params: List[String], label: String)
  object Ctor {
    def unapply[A](t: Type[A]): Option[(List[String], String)] = Some((List(tpe(using t)), "ctor"))
  }
}

trait FoHierUse { self: FoDeriv =>
  def names[A, B](a: Type[A], b: Type[B]): String = (a, b) match {
    case (SH(En(xs)), SH(En(ys))) => (xs ++ ys).mkString(",")
    case _ => "none"
  }
  def ctorOf[A](a: Type[A]): String = a match {
    case Ctor(ps, label) => label + ps.mkString
    case _ => "none"
  }
  def wrapped(es: List[ExistentialType]): List[String] = es.map { (value: ExistentialType) =>
    import value.{Underlying as V}
    val o = new Opt[V] {
      def of(value: Type[V]): String = tpe(using value) + "/" + tpe(using summon[Type[V]])
    }
    o.of(value.Underlying)
  }
  abstract class Opt[V] { def of(value: Type[V]): String }
}

trait FoPaths {
  final class Path private (private val segments: Vector[String]) {
    override def toString: String = segments.mkString("_", ".", "")
  }
  object Path {
    val Root: Path = new Path(Vector.empty)
    def select(p: Path, name: String): Path = new Path(p.segments :+ name)
    object AtField {
      def unapply(p: Path): Option[(String, Path)] = p.segments.headOption.map(h => (h, new Path(p.segments.tail)))
    }
  }
}

trait FoMod { self: FoDeriv =>
  def pair(k: ExistentialType, v: ExistentialType): Option[(ExistentialType, ExistentialType)] =
    Some[(ExistentialType.Bounded[Nothing, Any], ExistentialType.Bounded[Nothing, Any])](k -> v)
  def show(e: ExistentialType): String = tpe(using e.Underlying)
  case class Wrap[A](label: String, t: Type[A])
  def castK(e: ExistentialType): String = {
    val w = Some(e).map { x => x.asInstanceOf[Existential.Bounded[Nothing, Any, Type]].mapK[Wrap](_ => t => Wrap("w", t)) }.get
    w.value.label + tpe(using w.value.t)
  }
  object Single {
    def unapply[A](t: Type[A]): Option[String] = Some(tpe(using t))
  }
  def viaMatch(o: Option[ExistentialType]): String = o match {
    case Some(x) =>
      val Single(s) = x.Underlying: @unchecked
      s
    case None => "none"
  }
}

trait FoOuters { self: FoDeriv =>
  abstract class Tot[A](implicit ev: Type[A]) {
    def label: String
    def show: String = label + ":" + tpe(using ev)
  }
}

trait FoMod2 { self: FoDeriv =>
  def mk[A](implicit t: Type[A]): String = {
    val made = new Tot[A]()(using t) { def label = "tot" }
    made.show
  }
}

final class FoImpl extends FoDeriv {
  type Type[A] = String
  def tpe[A](using t: String): String = t
  protected object SH extends SHModule {
    def parse[A](implicit t: Type[A]): Option[En[A]] = Some(En(List(t)))
  }
}

// An inline method reading its class's parameter, reached before anything else of the class.
final class FoInto[A](val source: A, flag: Boolean) {
  inline def get: A = source
  inline def both: String = source.toString + flag
}

// A covariant parameter inherited at three instantiations: the base type is their meet, so the
// override through the intersection alias is one member with the trait's method.
trait FoCap
trait FoEff[F[_]]
class FoReq[T, -R](val v: T)
trait FoBackend[F[_], +P] {
  def send[T](r: FoReq[T, P & FoEff[F]]): String
}
trait FoPlain[F[_]] extends FoBackend[F, Any]
trait FoCapBackend[F[_]] extends FoPlain[F] with FoBackend[F, FoCap]
abstract class FoAbstract[F[_], S] extends FoBackend[F, S & FoCap] with FoCapBackend[F] {
  type R = S & (FoCap & FoEff[F])
  override def send[T](r: FoReq[T, R]): String = "sent " + r.v
}
final class FoConcrete extends FoAbstract[Option, Nothing]

// A nested match in a case body names the enclosing case's type variables (`T2$1` of
// `FoMapped[T$1, T2$1, R$1](raw, g)` inside `case (a, n) =>`), which stay the case's.
sealed trait FoGR[+T, -R]
case class FoMapped[T, T2, R](raw: FoGR[T, R], g: (T, Int) => T2) extends FoGR[T2, R]
case class FoBoth[A, B, R](l: FoGR[A, R], r: FoGR[B, Any]) extends FoGR[(A, Option[B]), R]
case object FoIgnore extends FoGR[Unit, Any]
case class FoLeaf[T](v: T) extends FoGR[T, Any]
class FoRunner {
  def run[T](r: FoGR[T, ?]): Option[(T, Int)] = (r, 1) match {
    case (FoMapped(raw, g), _) => run(raw).flatMap { case (a, n) => Some(g(a, n)).map((_, n + 1)) }
    case (FoBoth(l, r), _) => run(l).flatMap { case (a, n) => run(r).map { case (b, m) => ((a, Some(b)), n + m) } }
    case (FoIgnore, _) => Some(((), 0))
    case (FoLeaf(v), _) => Some((v, 0))
  }
}

// scala-library's small map constructed through `Predef.Map`, which the std's `Map` stands for.
object FoMaps {
  def single[K, V](k: K, v: V): Map[K, V] = new Map.Map1(k, v)
}
