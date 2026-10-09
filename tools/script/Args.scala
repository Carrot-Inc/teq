// A script's arguments: flags (`--name`), options with a value (`--name value`, `--name=value`) and
// positionals, against a usage line printed on a mistake (exit 2). `--teq <binary>` is every
// script's: the compiler under test, which wins over the variable `TEQ`.

final class Args(argv: Seq[String], usage: String):
  private var rest = argv.toVector
  private val seen = scala.collection.mutable.Set.empty[String]

  // The usage on stderr, status 2.
  def fail(why: String = null): Nothing =
    if why != null then System.err.println(why)
    System.err.println("usage: " + usage)
    Script.exit(2)

  // Whether the flag is there; taken out of the rest.
  def flag(name: String): Boolean =
    seen += name
    val at = rest.indexOf("--" + name)
    if at < 0 then false
    else
      rest = rest.patch(at, Nil, 1)
      true

  // The option's value, if given; taken out of the rest.
  def option(name: String): Option[String] =
    seen += name
    val at = rest.indexWhere(a => a == "--" + name || a.startsWith("--" + name + "="))
    if at < 0 then None
    else
      val a = rest(at)
      if a.contains('=') then
        rest = rest.patch(at, Nil, 1)
        Some(a.substring(a.indexOf('=') + 1))
      else if at + 1 < rest.length then
        val v = rest(at + 1)
        rest = rest.patch(at, Nil, 2)
        Some(v)
      else fail(s"--$name takes a value")

  // The compiler under test: `--teq`, else `TEQ`, else the default.
  def teq(default: String): String = option("teq").orElse(Option(System.getenv("TEQ")).filter(_.nonEmpty)).getOrElse(default)

  // What is left once the flags and options are taken: the positionals; an unknown `--x` is a
  // mistake.
  def positionals: Vector[String] =
    rest.find(a => a.startsWith("--") && a != "--").foreach(a => fail(s"unknown option $a"))
    rest.filter(_ != "--")

  // Exactly `n` positionals, else the usage.
  def exactly(n: Int): Vector[String] =
    val p = positionals
    if p.length != n then fail()
    p

object Args:
  def apply(argv: Array[String], usage: String): Args = new Args(argv.toSeq, usage)
