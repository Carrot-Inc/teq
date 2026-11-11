trait Outer:
  object Inner:
    sealed trait Bounded[F[_]]:
      def name: String
      def again[G[_]]: Bounded[G] = Bounded[G](name)
    object Bounded:
      final case class Impl[F[_]](name: String) extends Bounded[F]
      def apply[F[_]](name: String): Bounded[F] = Impl[F](name)

object Use extends Outer

trait Types:
  type Type[A]
  object Type:
    def apply[A](using t: Type[A]): Type[A] = t

trait SelfTyped { this: Types =>
  object Nested:
    class Holder[A](val tag: Type[A])
    object Holder:
      def make[A: Type]: Holder[A] = new Holder[A](Type[A])
}

trait Inherited extends Types {
  object Nested:
    class Holder[A](val tag: Type[A])
    object Holder:
      def make[A: Type]: Holder[A] = new Holder[A](Type[A])
}

object BothSelf extends Types with SelfTyped:
  type Type[A] = String
  given anyType[A]: Type[A] = "self"

object BothInherited extends Inherited:
  type Type[A] = String
  given anyType[A]: Type[A] = "inherited"

def viaSelf: String =
  import BothSelf.given
  BothSelf.Nested.Holder.make[Int].tag

def viaInherited: String =
  import BothInherited.given
  BothInherited.Nested.Holder.make[Int].tag

@main def run(): Unit =
  println(Use.Inner.Bounded[List]("made").again[Option].name)
  println(viaSelf)
  println(viaInherited)
