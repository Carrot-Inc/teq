package plw
object Mid:
  def forward(f: [A >: String] => (a: A) => A): [A >: String] => (a: A) => A = Api.keep(f)
