package nbd
object Mid:
  def forward(f: AnyRef { def outer(c: Ctx)(x: c.T): Parent[c.T] { def run[A <: c.T](a: A): A } }): AnyRef { def outer(c: Ctx)(x: c.T): Parent[c.T] { def run[A <: c.T](a: A): A } } = Api.keep(f)
