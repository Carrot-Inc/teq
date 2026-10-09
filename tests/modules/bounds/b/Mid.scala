package bnd
object Mid:
  def forward(c: Ctx)(f: Parent[c.T] { def run[A <: c.T](a: A): A }): Parent[c.T] { def run[A <: c.T](a: A): A } = Api.keep(c)(f)
