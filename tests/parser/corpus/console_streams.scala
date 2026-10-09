// scala.Console's streams as zio's `Console` writes through them: `out` and `err` are the
// system's, `withOut` names the stream `print` and `println` write to for the length of its body.
object Main:
  def main(args: Array[String]): Unit =
    println(Console.out eq System.out)
    println(Console.err eq System.err)
    Console.withOut(System.out) {
      Console.print("inside ")
      Console.println("withOut")
    }
    val n = Console.withOut(Console.out) { Console.println("nested"); 3 }
    println(n)
    Console.withErr(System.err)(Console.println("err left alone"))
    Console.out.println("through out")
    Console.flush()
