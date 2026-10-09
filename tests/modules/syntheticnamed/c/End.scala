package snm
object End:
  def use(f: [A] => (x$1: A) => A): String = Mid.forward(f)
  def main(args: Array[String]): Unit = println(use([A] => (x$1: A) => x$1))
