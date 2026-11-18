// System.out is a java.io.PrintStream over the process's output: a print stream over it writes there, bytes written
// to it are decoded as UTF-8, and System.setOut replaces it.
import java.io.PrintStream
@main def run(): Unit =
  val through = new PrintStream(System.out)
  through.print("through|")
  through.write(65)
  through.println("|end")
  through.flush()
  System.out.write(Array[Byte](104, 105, 10))
  System.out.flush()
  System.out.print(1)
  System.out.print('c')
  System.out.println(2.5)
  System.out.println("café")
  val saved = System.out
  System.setOut(new PrintStream(saved) { override def println(x: String): Unit = super.println("[set] " + x) })
  System.out.println("after setOut")
  System.setOut(saved)
  System.out.println("restored")
