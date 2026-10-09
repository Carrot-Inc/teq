package pvl
object End:
  def use(f: PolyFunction { def apply[A](a: A): A; val tag: Int }) = Mid.forward(f)
