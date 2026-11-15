import probe.*
@main def run(): Unit =
  println(s"${cons(0)} ${Log.take()}")
  println(s"${cons(1)} ${Log.take()}")
  println(s"${cons(2)} ${Log.take()}")
  println(s"${cons(3)} ${Log.take()}")
  println(s"${consAssigning()} ${Log.take()}")
  val none: Holder = null
  try println(consSelected(none))
  catch case _: Throwable => println(s"caught ${Log.take()}")
  println(s"${consSelected(new Holder(new Bag(5)))} ${Log.take()}")
  println(s"${consPure(0)} ${Log.take()}")
  println(s"${consPure(3)} ${Log.take()}")
  println(s"${consTwo(0)} ${Log.take()}")
  println(s"${consTwo(3)} ${Log.take()}")
  println(s"${consUsing(0)} ${Log.take()}")
  println(s"${consUsing(3)} ${Log.take()}")
  println(s"${all(3)} ${Log.take()}")
