// `scala.Console` with the ANSI colours of `scala.io.AnsiColor`, reached by the simple name and
// by the `_root_` path a library body spells, and `valueOf` of a literal type, a type alias
// and a class's type member.
trait Flags:
  final type debug = false
  inline def enabled: Boolean = valueOf[Flags#debug]

object Flags extends Flags

type Five = 5

object Main:
  def main(args: Array[String]): Unit =
    println(Console.RED.length)
    println(_root_.scala.Console.RESET == "\u001b[0m")
    println(s"[${Console.BOLD}x${Console.RESET}]".length)
    println(scala.io.AnsiColor.GREEN_B.drop(1))
    Console.println("to the console")
    Console.println()
    println(valueOf[3] + 1)
    println(valueOf[Five])
    println(Flags.enabled)
