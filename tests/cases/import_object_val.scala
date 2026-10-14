// An import of an object's own val in its body: the qualifier typed as the path `page.this.ui`, as
// dotty types it, the members read on the val.
object page:
  class Layout(n: Int):
    def hello = s"hi $n"
    val k = n + 1
  val ui = Layout(3)
  import ui.*
  val greeting = hello + k
  def later = hello.length
@main def main() = println((page.greeting, page.later))
