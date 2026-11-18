package pstraits
import java.io.PrintStream
// A library's traits over java.io.PrintStream that reach its printf and format through super. scalac compiles them
// (the super accessors are the traits' own members) and refuses a class that mixes one in: after ElimRepeated the
// JDK's members take an array, the accessors a Seq, so ResolveSuper finds no implementation.
trait Shouting extends PrintStream:
  def shout(s: String): PrintStream = super.printf("%s!", s)
trait Asking extends PrintStream:
  override def format(f: String, a: Any*): PrintStream = { print("[Asking]"); super.format(f, a*) }
