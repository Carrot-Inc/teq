package snm
object Mid:
  def forward(f: [A] => (x$1: A) => A): String = Api.keep(f)[String](x$1 = "ok")
