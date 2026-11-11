trait Outer:
  object Inner:
    trait Base[T]:
      def value: T
    def make[A](a: A): Base[A] = new Impl[A](a)
    object Base:
      def of[A](a: A): Base[A] = new Impl[A](a)
    trait Kinded[L, U >: L, F[_ >: L <: U]]:
      def held: F[L]
    def kinded[L, U >: L, F[_ >: L <: U]](a: F[L]): Kinded[L, U, F] = new KindedImpl[L, U, F](a)
  class Impl[A](val value: A) extends Inner.Base[A]
  class KindedImpl[L, U >: L, F[_ >: L <: U]](val held: F[L]) extends Inner.Kinded[L, U, F]

object Use extends Outer

@main def run(): Unit =
  println(Use.Inner.make(1).value)
  println(Use.Inner.Base.of("two").value)
  println(Use.Inner.kinded[Int, Any, List](List(3)).held)
