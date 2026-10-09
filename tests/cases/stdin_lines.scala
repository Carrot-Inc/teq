//> using platform jvm
// `System.in` against the JDK: read as lines, then to its end. Run as every case is, without
// arguments, it reads nothing; tests/run_interp.sh runs it again with `read` and two lines and a
// line without its end on stdin, where it prints each line and then the count of what remained.
import java.io.{BufferedReader, InputStreamReader}

object Main:
  def main(args: Array[String]): Unit =
    if args.isEmpty then println("no input read")
    else
      val in = new BufferedReader(new InputStreamReader(System.in))
      println("first: " + in.readLine())
      println("second: " + in.readLine())
      val rest = new StringBuilder
      var c = in.read()
      while c >= 0 do
        rest.append(c.toChar)
        c = in.read()
      println("rest: " + rest + " (" + rest.length + ")")
      println("after the end: " + in.readLine() + " " + System.in.read())
