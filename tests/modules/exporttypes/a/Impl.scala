package eta

object Impl:
  class C
  type T = Int
  type Box[X] = List[X]

object Api:
  export Impl.C
  export Impl.T
  export Impl.Box
