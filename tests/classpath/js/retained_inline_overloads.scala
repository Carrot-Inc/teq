// jars: fixtures
// targets: js interp jvm
// Two inline overrides of one name keep a retained body each, told apart by their erased
// signatures: a call through the trait runs the one of its overload.
import fix.retain.{Fmt, Loud}

@main def main(): Unit =
  val f: Fmt = Loud
  println(f.show(41))
  println(f.show("abc"))
  println(Loud.show(1))
