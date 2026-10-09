package facade

@jsImport("node:path", "join")
def join(parts: String*): String

@jsImport("node:util", "format")
def format(template: String, args: Any*): String

@jsImport("node:path", "sep")
val separator: String

object Paths:
  def under(root: String, names: List[String]): List[String] = names.map(n => join(root, n))
