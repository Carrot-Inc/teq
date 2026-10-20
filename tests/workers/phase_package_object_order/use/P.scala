package object p {
  val a: Int = counter.M.next
  object Q {
    val b: Int = counter.M.next
  }
  val c: Int = counter.M.next
}
