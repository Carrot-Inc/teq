object Opaque:
  opaque type T = Int
@main def opaqueSymbol(): Unit = println(OpaqueReflect.flags[Opaque.T])
