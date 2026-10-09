// `getClass` on a value whose static type is a primitive is the primitive's class, as scalac
// rewrites it (`1.getClass` prints `int`, `().getClass` `void`), the receiver run for its effects,
// and a primitive's `Class` prints its bare name (a boxed `Byte` is an `Integer` on JavaScript,
// where a number has no width: not tested here).
object Main:
  def f(): Byte = { println("eff"); 1 }
  def g(): Unit = println("unit eff")
  def main(args: Array[String]): Unit =
    println(f().getClass)
    println((1: Byte).getClass)
    println(List(1: Byte).head.getClass)
    println("s".getClass)
    println(g().getClass)
    println(classOf[Byte])
    println(classOf[Unit])
    println(true.getClass)
    println((1L).getClass)
