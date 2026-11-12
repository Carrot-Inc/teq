// `Nil` equals an empty sequence of another class, as every `Seq` does: its `equals` is `Seq`'s, which `List`'s
// class ancestors carry. In the modules of the split output `Nil` is registered before them
// (tests/cases/case_object_equals_from_superclass_trait).
@main def run(): Unit =
  println(Nil == Vector.empty[Int])
  println(Vector.empty[Int] == Nil)
  println(List(1, 2) == Vector(1, 2))
  println(Nil == Nil)
  println((Nil: Any).equals(List()))
  println(Nil.hashCode == Vector.empty[Int].hashCode)
  println(Nil.toString)
