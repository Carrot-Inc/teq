package clonediamond

class Diamond extends Left with Right

object Main:
  def via(x: Root): CharSequence = x.clone()
  def main(args: Array[String]): Unit =
    println(via(new Diamond))
    println(new Diamond().clone())
