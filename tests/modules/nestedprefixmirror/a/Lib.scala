package npm

// The children of a sealed parent through a prefix in an upstream module's products: `@Child`
// is written for `extends mid.T` and for a child in an object of the class (`extends
// mid.deep.U`), registered with the parent's class whatever its prefix, and read back the same.
class O(val n: Int):
  class Mid:
    sealed trait T
    class Deep:
      sealed trait U
    val deep = new Deep
  val mid = new Mid
  case class C(k: Int) extends mid.T
  object Kids:
    case class D(k: Int) extends mid.deep.U
