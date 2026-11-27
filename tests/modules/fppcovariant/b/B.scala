package covariant
object Run { def main(args: Array[String]): Unit = println(new K().isInstanceOf[Base[?]]) }
