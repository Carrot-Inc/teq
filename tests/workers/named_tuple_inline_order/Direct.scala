object Direct:
  def size: Int =
    val nt = (label = "b", active = true, count = 2)
    scala.NamedTupleDecomposition.size(nt)
