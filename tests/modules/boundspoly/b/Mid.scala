package bnp
object Mid:
  def forward(c: Ctx): [A <: c.T] => (a: A) => a.type = Api.inferred(c)
