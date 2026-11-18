object O1 {
  val a = O2.b
  val c = 1
}

object O2 {
  val b = O1.c
}

object M {
  var count = 0
  val xs = List(1, 2) match {
    case h :: _ => h + N.v
    case _ => 0
  }
}

object N {
  val v = M.count
}
