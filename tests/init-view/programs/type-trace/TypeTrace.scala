class TypeTrace {
  val a = {
    type T = b.type
    println("keep block")
    0
  }
  val b: AnyRef = new Object
}
