
case class A(a: Int, b: String)
object Main:
  def main(args: Array[String]): Unit =
    println(fields2[A])
    println(fields3[A])
    println(fields4[A])
    println(fields[A])
