package fix.cake

// Some of chimney's forms: an existential's member read through a value, protected members
// from an object nested in a self-typed trait, an inherited alias over the self type's member,
// an extractor object nested in an implicit class, a case class nested in an inner object under
// a type test, and the enclosing instance of a class nested in a class when it is constructed
// or extended beside its outer (`new Ty.Cache`, `object Two extends Helpers.Base`).
trait ExTypes:
  type Type[A]
  protected def show[A](t: Type[A]): String

trait ExExs { this: ExTypes =>
  protected object Existential:
    sealed trait Bounded[L, U >: L, F[_ >: L <: U]]:
      type Underlying >: L <: U
      implicit val Underlying: Type[Underlying]
      val value: F[Underlying]
    def apply[F[_], A](value0: F[A])(implicit t: Type[A]): Bounded[Nothing, Any, F] =
      new Bounded[Nothing, Any, F] { type Underlying = A; val Underlying = t; val value = value0 }
  protected object ExistentialType:
    type UpperBounded[U] = Existential.Bounded[Nothing, U, Type]
  type ?<[U] = ExistentialType.UpperBounded[U]
}

trait ExUses { this: ExTypes & ExExs =>
  def need[A](t: Type[A]): Type[A] = t
  def go(e: ?<[Any]): String = show(need(e.Underlying))
  object Inner:
    def run(e: ?<[Any]): String = show(e.Underlying)
}

final class ExImpl extends ExTypes, ExExs, ExUses:
  type Type[A] = String
  protected def show[A](t: String): String = "<" + t + ">"
  def make: ?<[Any] = Existential[Type, Int]("int")("Int")

trait PatFlags:
  implicit class FlagInterpolator(sc: StringContext):
    object flag:
      def unapplySeq(s: String): Option[Seq[String]] =
        if s.startsWith(sc.parts.head) then Some(Seq(s.drop(sc.parts.head.length))) else None
  def read(s: String): String = s match
    case flag"on=$v" => "flag " + v
    case other => "other " + other

trait NameDefs:
  protected val Namer: NamerModule
  protected trait NamerModule { this: Namer.type =>
    sealed trait Strategy
    object Strategy:
      final case class FromPrefix(src: String) extends Strategy
      case object FromType extends Strategy
    def name(s: Strategy): String
  }
  def fresh(prefix: Option[String]): String =
    Namer.name(prefix.fold(Namer.Strategy.FromType)(Namer.Strategy.FromPrefix(_)))

trait NameDefsPlatform extends NameDefs:
  protected object Namer extends NamerModule:
    def name(s: Strategy): String = s match
      case Strategy.FromPrefix(src) => src + "$1"
      case Strategy.FromType => "tpe$1"

final class PatImpl extends PatFlags, NameDefsPlatform

trait SupP:
  def tag: String
  object Helpers:
    abstract class Base(n: Int):
      def show: String = SupP.this.tag + n
  object Lits:
    object One extends Helpers.Base(1)
  object Two extends Helpers.Base(2)

trait CacheTypes:
  def tag: String
  val Ty: TyModule
  trait TyModule:
    def name: String
    class Cache(n: Int):
      def show: String = tag + name + n

trait CacheTypesPlatform extends CacheTypes:
  object Ty extends TyModule:
    def name = "ty"

trait CacheUses { this: CacheTypes =>
  private val cache = new Ty.Cache(1)
  def cached: String = cache.show
}

final class SupImpl extends SupP, CacheTypesPlatform, CacheUses:
  def tag = "t:"
