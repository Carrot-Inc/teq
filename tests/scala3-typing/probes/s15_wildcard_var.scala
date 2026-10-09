class Box[T](var v: T)
@main def run(): Unit =
  val s = Box[String]("")
  val i = Box[Int](3)
  var box: Box[?] = s
  val sv = box.v
  box = i
  box.v = sv
  val c: Int = i.v
  println(c + 1)
