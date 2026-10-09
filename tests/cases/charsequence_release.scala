// teq: --release
// `CharSequence` members on a JS string and on the string builders under `--release`: the
// trait's names stay their own, since a string answers them natively and the std's templates
// name them (scala-java-time's parsers read their input as a `CharSequence`).
object Main:
  def describe(cs: CharSequence): String =
    "" + cs.length + ":" + cs.charAt(0) + ":" + cs.subSequence(1, 3) + ":" + cs.isEmpty + ":" + cs.charAt(cs.length - 1)
  def main(args: Array[String]): Unit =
    println(describe("hello"))
    println(describe(new java.lang.StringBuilder("world")))
    println(describe(new StringBuilder("scala")))
