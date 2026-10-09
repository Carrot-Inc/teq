package bni
object Mid:
  def forward(c: Ctx): Parent[c.T] { def run[A <: c.T](a: A): A } = Api.inferred(c)
