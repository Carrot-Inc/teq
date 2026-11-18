class TypeAlias {
  val a = {
    type T = b.type
    0
  }
  val b: AnyRef = new Object
}
