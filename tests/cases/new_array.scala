@main def newArray(): Unit =
  val ints = new Array[Int](3)
  ints(1) = 7
  println(ints.toList)
  val strs = new Array[String](2)
  println(strs.toList)
  println(new Array[Boolean](2).toList)
  println(new Array[Long](1).toList)
  println(new Array[Char](1).toList.map(_.toInt))
  val copy = ints.clone()
  copy(0) = 1
  println(s"${ints.toList} ${copy.toList}")
