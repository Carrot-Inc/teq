// A block's type alias is completed under the imports before it (dotty's `Namer.index`, whose
// completers keep their definition's import context), named from anywhere in the block.
object O:
  type F = Int
  val v = "v"

@main def run(): Unit =
  import O.F
  type R = F
  val r: R = 1
  def early: Later = List(r)
  import O.v
  type Later = List[F]
  type x = { type l = (F, v.type) }
  val pair: x#l = (2, v)
  println(s"$r ${early} $pair")
