// `isInstanceOf` of an abstract type member erases to the member's bound, as a newtype
// encoding relies on (cats' `NonEmptySet` is `NonEmptySetImpl.Type[A]`): Magnolia tests a
// field's default value against the field's type this way, under `@unchecked`.
object Wrapped:
  type Type[A] <: Set[A]
  def create[A](s: Set[A]): Type[A] = s.asInstanceOf[Type[A]]
  type Tag <: AnyRef
  def tag(s: String): Tag = s.asInstanceOf[Tag]

object Main:
  inline def defaultAs[p](evaluator: () => Any): Option[p] =
    val v = evaluator()
    if (v: @unchecked).isInstanceOf[p] then Some(v.asInstanceOf[p]) else None
  def main(args: Array[String]): Unit =
    println(defaultAs[Wrapped.Type[Int]](() => Wrapped.create(Set(1, 2))).map(_.size))
    println(defaultAs[Wrapped.Type[Int]](() => "no").isDefined)
    println(defaultAs[Wrapped.Tag](() => Wrapped.tag("t")))
    println(defaultAs[Wrapped.Tag](() => List(1)).isDefined)
    println((Wrapped.create(Set("a")): Any) match { case s: Wrapped.Type[String] @unchecked => s.size; case _ => -1 })
    println(("a": Any) match { case s: Wrapped.Type[String] @unchecked => s.size; case _ => -1 })
