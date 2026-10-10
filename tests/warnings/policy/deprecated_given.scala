trait Box[A] { def value: Int }
object Lib { @deprecated("old given", "1") given box[A]: Box[A] with { def value = 1 } }
object Main {
 import Lib.given
 def main(args: Array[String]): Unit = println(summon[Box[Int]].value)
}
