package main
object Ext { def left(): Int = { println("left"); 0 } }
val own: List[Int] = { println("init main"); List(3) }
def viaDef(): List[Int] = Ext.left() :: own
@main def run(): Unit =
  println("start")
  println(new one.C().f())
  println(two.cons(Ext.left()))
  println(viaDef())
