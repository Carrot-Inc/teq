// The operations of an indexed sequence give indexed sequences, where a library's signature
// expects one (utest's `Tests.++` concatenates two into a `Right`).
@main def run =
  val a: IndexedSeq[Int] = IndexedSeq(1, 2)
  val b: IndexedSeq[Int] = a ++ IndexedSeq(3)
  val c: IndexedSeq[Int] = b.map(_ * 2).filter(_ > 2)
  val d: IndexedSeq[String] = (1 to 3).map(_.toString)
  val e: Either[Any, IndexedSeq[Int]] = Right(a ++ b)
  val f: IndexedSeq[Int] = a.reverse.appended(0).take(2)
  println(b)
  println(c)
  println(d)
  println(e)
  println(f)
