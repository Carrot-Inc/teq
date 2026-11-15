import lib.*
@main def run(): Unit =
  val plain = List(0)
  println(s"${onList(plain)} ${Log.take()}")
  println(s"${onList(lz)} ${Log.take()}")
  val acc = new Acc(plain)
  println(s"${acc.add()} ${Log.take()}")
  println(s"${acc.pre()} ${Log.take()}")
