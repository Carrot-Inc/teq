package pvl
object Mid:
  def forward(f: PolyFunction { def apply[A](a: A): A; val tag: Int }): PolyFunction { def apply[A](a: A): A; val tag: Int } = Api.keep(f)
