// An iterator over an array reads the array: what is written to the array after the iterator
// was made is what the iterator gives, and so for a view and for a sequence that wraps the
// array without a copy.
@main def run(): Unit =
  val a = Array(1, 2, 3)
  val it = a.iterator
  a(0) = 9
  println(it.next())
  a(2) = 7
  println(it.toList)
  val wrapped = scala.collection.immutable.ArraySeq.unsafeWrapArray(a)
  val over = wrapped.iterator
  a(0) = 5
  println(over.next().toString + " " + wrapped(0) + " " + wrapped.mkString(","))
  val view = a.view
  a(1) = 8
  println(view.toList)
  val words = Array("a", "b", "c")
  val letters = words.iterator
  words(1) = "z"
  println(letters.mkString)
  val doubles = Array(1.5, 2.5)
  val halves = doubles.iterator.map(_ * 2)
  doubles(0) = 4.5
  println(halves.toList.map(_.toInt))
  val copy = a.toSeq
  val list = a.toList
  a(0) = 0
  println(copy.mkString(",") + " " + list.mkString(",") + " " + a.mkString(","))
