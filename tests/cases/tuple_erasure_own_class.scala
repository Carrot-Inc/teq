// A class of the program named `Tuple` in a package named `scala` below the root keeps its own erasure:
// only the root `scala`'s tuple supertypes erase to `Product` and `Object`.
package example.scala

class Tuple(val n: Int)

object Main:
  def echo(x: Tuple): Tuple = x
  def main(args: Array[String]): Unit =
    println(echo(new Tuple(7)).n)
