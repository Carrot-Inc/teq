type Element[T] = T match {
  case List[value] => Element[value]
  case _ => T
}

@main def finiteMatchReduction(): Unit = {
  val deep: Element[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[List[Int]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]] = 7
  val next: Element[List[String]] = "restored"
  println(deep)
  println(next)
}
