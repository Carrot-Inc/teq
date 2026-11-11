import Shape.*

inline def plain(x: Int): Int = x + 1
inline def byname(x: => Int): Int = x
inline def keep(inline x: Int): Int = x
def fresh(): Int = 3

@main def run(): Unit =
  val k = 2
  println(inspect(k + plain(k)))
  println(inspect(byname(fresh())))
  println(inspect(keep(k + plain(k))))
  println(inspect(mapped(k + plain(k))))
  println(inspect(emptyBlock(k)))
  println(blocks(byname(fresh())))
  println(blocks(mapped(plain(k))))
  println(blocks(emptyBlock(k)))
