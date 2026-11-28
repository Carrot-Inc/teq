object Main {
 def arg(): Any = { println("arg"); "x" }
 def main(args: Array[String]): Unit = {
  val result = arg() == arg()
  scala.runtime.equal("x", "x")
  println(result)
 }
}
