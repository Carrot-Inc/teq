class TypeSelect {
  class Holder { type T = Int }
  val a = {
    type T = b.T
    0
  }
  val b = new Holder
}
