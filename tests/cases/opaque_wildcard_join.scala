object Wrap {
  opaque type Broad >: Tuple <: Any = Tuple
  opaque type Key[N <: Tuple, +V <: Tuple] >: V <: Broad = V
  def key[N <: Tuple, V <: Tuple](value: V): Key[N, V] = value
}

object Main {
  def main(args: Array[String]): Unit = {
    var combined = if args.isEmpty then
      Wrap.key[("left", "right"), (Int, Int)]((1, 2))
    else
      Wrap.key[("first", "last"), (Int, Int)]((3, 4))
    val broad: Wrap.Broad = combined
    combined = broad
    println("opaque join")
  }
}
