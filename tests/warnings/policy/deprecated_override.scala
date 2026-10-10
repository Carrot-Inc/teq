class A { def f: Int = 1 }
class B extends A { @deprecated("old", "1") override def f: Int = 2 }
object Main { def main(args: Array[String]): Unit = println("ok") }
