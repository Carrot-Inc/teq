package snu
object End:
  def use(f: [A] => A => A): String = Mid.forward(f)
  def main(args: Array[String]): Unit = println(use([A] => (a: A) => a))
