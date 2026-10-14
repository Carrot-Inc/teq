// A type lambda written as a refinement's member projected (`({ type L[V] = Option[M[V]] })#L`), the Scala 2
// spelling scalac 3.8.4 reads: inside the type arguments of `new` and of a parent, where a brace is a refinement
// and not the template's body (dotty's `simpleType1` under `typeArgs`).
trait Splittable[M[_]]:
  def map[A, B](inputs: M[A], project: A => B): M[B]
  def name: String

trait Ops[Self[+_], A]:
  def self: Self[A]

final class Keyed[K, V](val value: V) extends Ops[({ type Self[+X] = Keyed[K, X @scala.annotation.unchecked.uncheckedVariance] })#Self, V]:
  def self: Keyed[K, V] = this

object Splittable:
  given Splittable[List] with
    def map[A, B](inputs: List[A], project: A => B): List[B] = inputs.map(project)
    def name = "List"
  def optionOf[M[_]](using inner: Splittable[M]): Splittable[({ type OptionM[V] = Option[M[V]] })#OptionM] =
    new Splittable[({ type OptionM[V] = Option[M[V]] })#OptionM]:
      def map[A, B](inputs: Option[M[A]], project: A => B): Option[M[B]] = inputs.map(inner.map(_, project))
      def name = s"Option of ${inner.name}"

@main def main(): Unit =
  val s = Splittable.optionOf[List]
  println(s.name)
  println(s.map(Some(List(1, 2, 3)), _ * 2))
  println(s.map(None, (x: Int) => x))
  println(new Keyed[String, Int](7).self.value)
