package p

object Impl:
  val n: Int = 1
  object Inner:
    val k: Int = 2

// A stable value of the package, which a file import's path in the package starts with.
val p = Impl
